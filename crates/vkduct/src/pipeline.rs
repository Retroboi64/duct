use crate::types::*;
use crate::types::VkPipeline;
use crate::vk;
use crate::vk::*;
use duct_core::*;
use std::ffi::{c_char, c_void};

impl VkPipeline {
    pub(crate) fn get_or_create_graphics_pipeline(
        &mut self,
        pso: &DuctPipelineStateDesc,
        is_offscreen: bool,
    ) -> vk::VkPipeline {
        let dfns = match &self.device_fns {
            Some(d) if !self.device.is_null() => *d,
            _ => return VK_NULL_HANDLE,
        };

        let target_rp = if is_offscreen {
            if (pso.depth_test || pso.depth_write) && self.offscreen_render_pass_with_depth != VK_NULL_HANDLE {
                self.offscreen_render_pass_with_depth
            } else {
                self.offscreen_render_pass
            }
        } else {
            self.render_pass
        };
        if target_rp == VK_NULL_HANDLE || self.pipeline_layout == VK_NULL_HANDLE {
            return VK_NULL_HANDLE;
        }

        let attr_count = pso.vertex_layout.attribute_count;
        let stride = if pso.vertex_layout.stride > 0 {
            pso.vertex_layout.stride
        } else {
            12
        };

        let mut attr_formats = [0u32; 8];
        let mut attr_offsets = [0u32; 8];
        for i in 0..(attr_count as usize).min(8) {
            attr_formats[i] = map_vertex_format(pso.vertex_layout.attributes[i].format);
            attr_offsets[i] = pso.vertex_layout.attributes[i].offset;
        }

        let key = VkPipelineKey {
            vs: pso.vertex_shader.0,
            fs: pso.fragment_shader.0,
            cull_mode: pso.cull_mode as u32,
            blend_mode: pso.blend_mode as u32,
            depth_test: pso.depth_test,
            depth_write: pso.depth_write,
            depth_compare: pso.depth_compare as u32,
            wireframe: self.wireframe,
            is_offscreen,
            stride,
            attr_count,
            attr_formats,
            attr_offsets,
        };

        if let Some(&cached) = self.graphics_pipelines.get(&key) {
            return cached;
        }

        let vs_module = if let Some(rec) = self.shaders.get(&pso.vertex_shader) {
            if attr_count >= 2 && !rec.is_custom_spv && rec.module_with_color != VK_NULL_HANDLE {
                rec.module_with_color
            } else {
                rec.module
            }
        } else if attr_count >= 2 && self.default_vs_color_module != VK_NULL_HANDLE {
            self.default_vs_color_module
        } else {
            self.default_vs_module
        };

        let fs_module = if let Some(rec) = self.shaders.get(&pso.fragment_shader) {
            rec.module
        } else {
            self.default_fs_module
        };

        if vs_module == VK_NULL_HANDLE || fs_module == VK_NULL_HANDLE {
            return VK_NULL_HANDLE;
        }

        let entry_name = b"main\0";
        let shader_stages = [
            VkPipelineShaderStageCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                stage: VK_SHADER_STAGE_VERTEX_BIT,
                module: vs_module,
                pName: entry_name.as_ptr() as *const c_char,
                pSpecializationInfo: std::ptr::null(),
            },
            VkPipelineShaderStageCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                stage: VK_SHADER_STAGE_FRAGMENT_BIT,
                module: fs_module,
                pName: entry_name.as_ptr() as *const c_char,
                pSpecializationInfo: std::ptr::null(),
            },
        ];

        let binding_desc = VkVertexInputBindingDescription {
            binding: 0,
            stride,
            inputRate: VK_VERTEX_INPUT_RATE_VERTEX,
        };

        let mut attr_descs = Vec::new();
        if attr_count == 0 {
            attr_descs.push(VkVertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: VK_FORMAT_R32G32B32_SFLOAT,
                offset: 0,
            });
        } else {
            let max_attrs = (attr_count as usize).min(8);
            for i in 0..max_attrs {
                let a = &pso.vertex_layout.attributes[i];
                attr_descs.push(VkVertexInputAttributeDescription {
                    location: a.location,
                    binding: 0,
                    format: map_vertex_format(a.format),
                    offset: a.offset,
                });
            }
        }

        let vertex_input_info = VkPipelineVertexInputStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            vertexBindingDescriptionCount: 1,
            pVertexBindingDescriptions: &binding_desc,
            vertexAttributeDescriptionCount: attr_descs.len() as u32,
            pVertexAttributeDescriptions: attr_descs.as_ptr(),
        };

        let input_assembly = VkPipelineInputAssemblyStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            topology: VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
            primitiveRestartEnable: VK_FALSE,
        };

        let viewport_state = VkPipelineViewportStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            viewportCount: 1,
            pViewports: std::ptr::null(),
            scissorCount: 1,
            pScissors: std::ptr::null(),
        };

        let cull_mode = match pso.cull_mode {
            DuctCullMode::None => VK_CULL_MODE_NONE,
            DuctCullMode::Front => VK_CULL_MODE_FRONT_BIT,
            DuctCullMode::Back => VK_CULL_MODE_BACK_BIT,
        };

        let rasterizer = VkPipelineRasterizationStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            depthClampEnable: VK_FALSE,
            rasterizerDiscardEnable: VK_FALSE,
            polygonMode: VK_POLYGON_MODE_FILL,
            cullMode: cull_mode,
            // With negative viewport height (y = h, height = -h), +Y is up, and CCW triangles
            // in world/clip space are rasterized as clockwise in framebuffer space.
            frontFace: VK_FRONT_FACE_CLOCKWISE,
            depthBiasEnable: VK_FALSE,
            depthBiasConstantFactor: 0.0,
            depthBiasClamp: 0.0,
            depthBiasSlopeFactor: 0.0,
            lineWidth: 1.0,
        };

        let multisampling = VkPipelineMultisampleStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            rasterizationSamples: VK_SAMPLE_COUNT_1_BIT,
            sampleShadingEnable: VK_FALSE,
            minSampleShading: 1.0,
            pSampleMask: std::ptr::null(),
            alphaToCoverageEnable: VK_FALSE,
            alphaToOneEnable: VK_FALSE,
        };

        let depth_compare = match pso.depth_compare {
            DuctCompareOp::Never => VK_COMPARE_OP_NEVER,
            DuctCompareOp::Less => VK_COMPARE_OP_LESS,
            DuctCompareOp::Equal => VK_COMPARE_OP_EQUAL,
            DuctCompareOp::LessEqual => VK_COMPARE_OP_LESS_OR_EQUAL,
            DuctCompareOp::Greater => VK_COMPARE_OP_GREATER,
            DuctCompareOp::NotEqual => VK_COMPARE_OP_NOT_EQUAL,
            DuctCompareOp::GreaterEqual => VK_COMPARE_OP_GREATER_OR_EQUAL,
            DuctCompareOp::Always => VK_COMPARE_OP_ALWAYS,
        };

        let depth_stencil = VkPipelineDepthStencilStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            depthTestEnable: if pso.depth_test { VK_TRUE } else { VK_FALSE },
            depthWriteEnable: if pso.depth_write { VK_TRUE } else { VK_FALSE },
            depthCompareOp: depth_compare,
            depthBoundsTestEnable: VK_FALSE,
            stencilTestEnable: VK_FALSE,
            front: VkStencilOpState::default(),
            back: VkStencilOpState::default(),
            minDepthBounds: 0.0,
            maxDepthBounds: 1.0,
        };

        let blend_attachment = match pso.blend_mode {
            DuctBlendMode::Opaque => VkPipelineColorBlendAttachmentState {
                blendEnable: VK_FALSE,
                srcColorBlendFactor: VK_BLEND_FACTOR_ONE,
                dstColorBlendFactor: VK_BLEND_FACTOR_ZERO,
                colorBlendOp: VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VK_BLEND_FACTOR_ZERO,
                alphaBlendOp: VK_BLEND_OP_ADD,
                colorWriteMask: VK_COLOR_COMPONENT_ALL,
            },
            DuctBlendMode::AlphaBlend => VkPipelineColorBlendAttachmentState {
                blendEnable: VK_TRUE,
                srcColorBlendFactor: VK_BLEND_FACTOR_SRC_ALPHA,
                dstColorBlendFactor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
                colorBlendOp: VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
                alphaBlendOp: VK_BLEND_OP_ADD,
                colorWriteMask: VK_COLOR_COMPONENT_ALL,
            },
            DuctBlendMode::Additive => VkPipelineColorBlendAttachmentState {
                blendEnable: VK_TRUE,
                srcColorBlendFactor: VK_BLEND_FACTOR_SRC_ALPHA,
                dstColorBlendFactor: VK_BLEND_FACTOR_ONE,
                colorBlendOp: VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                alphaBlendOp: VK_BLEND_OP_ADD,
                colorWriteMask: VK_COLOR_COMPONENT_ALL,
            },
        };

        let color_blending = VkPipelineColorBlendStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            logicOpEnable: VK_FALSE,
            logicOp: 0,
            attachmentCount: 1,
            pAttachments: &blend_attachment,
            blendConstants: [0.0, 0.0, 0.0, 0.0],
        };

        let dynamic_states = [VK_DYNAMIC_STATE_VIEWPORT, VK_DYNAMIC_STATE_SCISSOR];
        let dynamic_state = VkPipelineDynamicStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            dynamicStateCount: dynamic_states.len() as u32,
            pDynamicStates: dynamic_states.as_ptr(),
        };

        let pipeline_info = VkGraphicsPipelineCreateInfo {
            sType: VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            stageCount: 2,
            pStages: shader_stages.as_ptr(),
            pVertexInputState: &vertex_input_info,
            pInputAssemblyState: &input_assembly,
            pTessellationState: std::ptr::null(),
            pViewportState: &viewport_state,
            pRasterizationState: &rasterizer,
            pMultisampleState: &multisampling,
            pDepthStencilState: &depth_stencil,
            pColorBlendState: &color_blending,
            pDynamicState: &dynamic_state,
            layout: self.pipeline_layout,
            renderPass: target_rp,
            subpass: 0,
            basePipelineHandle: VK_NULL_HANDLE,
            basePipelineIndex: -1,
        };

        let mut pipeline = VK_NULL_HANDLE;
        let res = unsafe {
            (dfns.CreateGraphicsPipelines)(
                self.device,
                VK_NULL_HANDLE,
                1,
                &pipeline_info,
                std::ptr::null(),
                &mut pipeline,
            )
        };
        if res == VK_SUCCESS && pipeline != VK_NULL_HANDLE {
            self.graphics_pipelines.insert(key, pipeline);
            pipeline
        } else {
            eprintln!("[vkduct][ERROR] vkCreateGraphicsPipelines failed with code {}", res);
            VK_NULL_HANDLE
        }
    }

    pub(crate) fn update_descriptor_set(&mut self) {
        let dfns = match &self.device_fns {
            Some(d) if !self.device.is_null() => *d,
            _ => return,
        };
        if self.descriptor_set == VK_NULL_HANDLE {
            return;
        }

        let default_tex = self.textures.get(&self.default_white_tex).cloned();

        let mut image_infos = [VkDescriptorImageInfo {
            sampler: VK_NULL_HANDLE,
            imageView: VK_NULL_HANDLE,
            imageLayout: 0,
        }; 24];
        let mut writes = Vec::with_capacity(24);

        for slot in 0..8u32 {
            let bound = self.bound_textures[slot as usize];
            let rec = self
                .textures
                .get(&bound)
                .cloned()
                .or_else(|| default_tex.clone());

            if let Some(t) = rec {
                if t.view != VK_NULL_HANDLE && t.sampler != VK_NULL_HANDLE {
                    let base_idx = (slot as usize) * 3;

                    // 1. Sampled Image (binding = 2*slot)
                    image_infos[base_idx] = VkDescriptorImageInfo {
                        sampler: VK_NULL_HANDLE,
                        imageView: t.view,
                        imageLayout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                    };
                    writes.push(VkWriteDescriptorSet {
                        sType: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                        pNext: std::ptr::null(),
                        dstSet: self.descriptor_set,
                        dstBinding: 2 * slot,
                        dstArrayElement: 0,
                        descriptorCount: 1,
                        descriptorType: VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                        pImageInfo: &image_infos[base_idx],
                        pBufferInfo: std::ptr::null(),
                        pTexelBufferView: std::ptr::null(),
                    });

                    // 2. Sampler (binding = 2*slot + 1)
                    image_infos[base_idx + 1] = VkDescriptorImageInfo {
                        sampler: t.sampler,
                        imageView: VK_NULL_HANDLE,
                        imageLayout: 0,
                    };
                    writes.push(VkWriteDescriptorSet {
                        sType: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                        pNext: std::ptr::null(),
                        dstSet: self.descriptor_set,
                        dstBinding: 2 * slot + 1,
                        dstArrayElement: 0,
                        descriptorCount: 1,
                        descriptorType: VK_DESCRIPTOR_TYPE_SAMPLER,
                        pImageInfo: &image_infos[base_idx + 1],
                        pBufferInfo: std::ptr::null(),
                        pTexelBufferView: std::ptr::null(),
                    });

                    // 3. Combined Image Sampler (binding = 16 + slot)
                    image_infos[base_idx + 2] = VkDescriptorImageInfo {
                        sampler: t.sampler,
                        imageView: t.view,
                        imageLayout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                    };
                    writes.push(VkWriteDescriptorSet {
                        sType: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                        pNext: std::ptr::null(),
                        dstSet: self.descriptor_set,
                        dstBinding: 16 + slot,
                        dstArrayElement: 0,
                        descriptorCount: 1,
                        descriptorType: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                        pImageInfo: &image_infos[base_idx + 2],
                        pBufferInfo: std::ptr::null(),
                        pTexelBufferView: std::ptr::null(),
                    });
                }
            }
        }

        if !writes.is_empty() {
            unsafe {
                (dfns.UpdateDescriptorSets)(
                    self.device,
                    writes.len() as u32,
                    writes.as_ptr(),
                    0,
                    std::ptr::null(),
                );
            }
        }
    }

    pub(crate) fn prepare_draw(&mut self) {
        let pso = self.current_pso.unwrap_or(DuctPipelineStateDesc {
            vertex_shader: DuctShaderHandle::INVALID,
            fragment_shader: DuctShaderHandle::INVALID,
            vertex_layout: DuctVertexLayout::new(12).add_attribute(0, DuctVertexFormat::Float3, 0),
            cull_mode: DuctCullMode::None,
            blend_mode: DuctBlendMode::AlphaBlend,
            depth_test: false,
            depth_write: false,
            depth_compare: DuctCompareOp::Less,
        });

        let is_offscreen = self.current_pass_is_offscreen;
        let pipeline = self.get_or_create_graphics_pipeline(&pso, is_offscreen);
        self.active_graphics_pipeline = pipeline;

        if self.descriptor_set_dirty {
            self.update_descriptor_set();
            self.descriptor_set_dirty = false;
        }

        if self.cmd_recording && self.render_pass_active && pipeline != VK_NULL_HANDLE {
            if let Some(dfns) = self.device_fns {
                unsafe {
                    (dfns.CmdBindPipeline)(
                        self.command_buffer,
                        VK_PIPELINE_BIND_POINT_GRAPHICS,
                        pipeline,
                    );
                    (dfns.CmdPushConstants)(
                        self.command_buffer,
                        self.pipeline_layout,
                        VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
                        0,
                        std::mem::size_of::<VkPushConstantBlock>() as u32,
                        &self.push_constants as *const VkPushConstantBlock as *const c_void,
                    );
                    if self.descriptor_set != VK_NULL_HANDLE && self.pipeline_layout != VK_NULL_HANDLE {
                        (dfns.CmdBindDescriptorSets)(
                            self.command_buffer,
                            VK_PIPELINE_BIND_POINT_GRAPHICS,
                            self.pipeline_layout,
                            0,
                            1,
                            &self.descriptor_set,
                            0,
                            std::ptr::null(),
                        );
                    }
                    if self.last_bound_vbo != VK_NULL_HANDLE {
                        let offset: VkDeviceSize = 0;
                        (dfns.CmdBindVertexBuffers)(
                            self.command_buffer,
                            0,
                            1,
                            &self.last_bound_vbo,
                            &offset,
                        );
                    }
                }
            }
        }
    }
    pub(crate) fn create_shader_impl(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return Err(DuctResult::ErrorInvalidParameter);
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut module = VK_NULL_HANDLE;
        let module_with_color = VK_NULL_HANDLE;
        let mut is_custom_spv = false;

        // 1. Check if caller provided raw SPIR-V binary (Magic 0x07230203)
        if slice.len() >= 4 && slice.len() % 4 == 0 {
            let magic = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
            if magic == 0x0723_0203 {
                is_custom_spv = true;
                if let Some(dfns) = &self.device_fns {
                    if !self.device.is_null() {
                        let create_info = VkShaderModuleCreateInfo {
                            sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            codeSize: slice.len(),
                            pCode: slice.as_ptr() as *const u32,
                        };
                        let _ = unsafe {
                            (dfns.CreateShaderModule)(
                                self.device,
                                &create_info,
                                std::ptr::null(),
                                &mut module,
                            )
                        };
                    }
                }
            }
        }

        // 2. If GLSL source string was provided, compile using Naga cross-compiler
        if !is_custom_spv {
            let src_str = match std::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("[vkduct][ERROR] Shader source is neither valid UTF-8 nor SPIR-V binary");
                    return Err(DuctResult::ErrorInvalidParameter);
                }
            };

            match duct_core::shader::glsl_to_spirv(src_str, desc.stage) {
                Ok(spv_words) => {
                    module = unsafe { self.create_spv_module(&spv_words) };
                    if module == VK_NULL_HANDLE {
                        eprintln!("[vkduct][ERROR] Failed to create Vulkan shader module from compiled SPIR-V");
                        return Err(DuctResult::ErrorInitializationFailed);
                    }
                    is_custom_spv = true;
                }
                Err(err) => {
                    eprintln!("[vkduct][ERROR] Shader compilation failed:\n{}", err);
                    return Err(DuctResult::ErrorInvalidParameter);
                }
            }
        }

        let handle = DuctShaderHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.shaders.insert(
            handle,
            VkShaderRecord {
                module,
                module_with_color,
                stage: desc.stage,
                is_custom_spv,
            },
        );
        Ok(handle)
    }

    pub(crate) fn reload_shader_impl(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        if !self.shaders.contains_key(&handle) {
            return DuctResult::ErrorResourceNotFound;
        }
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return DuctResult::ErrorInvalidParameter;
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut new_module = VK_NULL_HANDLE;
        let new_module_color = VK_NULL_HANDLE;
        let mut is_custom_spv = false;

        if slice.len() >= 4 && slice.len() % 4 == 0 {
            let magic = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
            if magic == 0x0723_0203 {
                is_custom_spv = true;
                if let Some(dfns) = &self.device_fns {
                    if !self.device.is_null() {
                        let create_info = VkShaderModuleCreateInfo {
                            sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            codeSize: slice.len(),
                            pCode: slice.as_ptr() as *const u32,
                        };
                        let _ = unsafe {
                            (dfns.CreateShaderModule)(
                                self.device,
                                &create_info,
                                std::ptr::null(),
                                &mut new_module,
                            )
                        };
                    }
                }
            }
        }

        if !is_custom_spv {
            let src_str = match std::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("[vkduct][ERROR] Shader reload source is neither valid UTF-8 nor SPIR-V binary");
                    return DuctResult::ErrorInvalidParameter;
                }
            };

            match duct_core::shader::glsl_to_spirv(src_str, desc.stage) {
                Ok(spv_words) => {
                    new_module = unsafe { self.create_spv_module(&spv_words) };
                    if new_module == VK_NULL_HANDLE {
                        eprintln!("[vkduct][ERROR] Failed to create Vulkan shader module from reloaded SPIR-V");
                        return DuctResult::ErrorInitializationFailed;
                    }
                    is_custom_spv = true;
                }
                Err(err) => {
                    eprintln!("[vkduct][ERROR] Shader reload compilation failed:\n{}", err);
                    return DuctResult::ErrorInvalidParameter;
                }
            }
        }

        if let Some(dfns) = &self.device_fns {
            if !self.device.is_null() {
                unsafe {
                    let _ = (dfns.DeviceWaitIdle)(self.device);
                    // Invalidate cached pipelines referencing this shader
                    let keys_to_remove: Vec<VkPipelineKey> = self
                        .graphics_pipelines
                        .keys()
                        .filter(|k| k.vs == handle.0 || k.fs == handle.0)
                        .copied()
                        .collect();
                    for k in keys_to_remove {
                        if let Some(p) = self.graphics_pipelines.remove(&k) {
                            if p != VK_NULL_HANDLE {
                                (dfns.DestroyPipeline)(self.device, p, std::ptr::null());
                            }
                        }
                    }
                    if let Some(rec) = self.shaders.get_mut(&handle) {
                        if rec.module != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(self.device, rec.module, std::ptr::null());
                        }
                        if rec.module_with_color != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(
                                self.device,
                                rec.module_with_color,
                                std::ptr::null(),
                            );
                        }
                        rec.module = new_module;
                        rec.module_with_color = new_module_color;
                        rec.stage = desc.stage;
                        rec.is_custom_spv = is_custom_spv;
                    }
                }
            }
        }

        println!("[vkduct] Shader {:?} hot-reloaded on Vulkan device.", handle);
        DuctResult::Success
    }

    pub(crate) fn destroy_shader_impl(&mut self, handle: DuctShaderHandle) -> DuctResult {
        if let Some(rec) = self.shaders.remove(&handle) {
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        let _ = (dfns.DeviceWaitIdle)(self.device);
                        if rec.module != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(self.device, rec.module, std::ptr::null());
                        }
                        if rec.module_with_color != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(
                                self.device,
                                rec.module_with_color,
                                std::ptr::null(),
                            );
                        }
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn set_pipeline_state_impl(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult {
        self.pipeline_switches_in_frame += 1;

        // Check if binding a Compute Shader
        if let Some(vs_rec) = self.shaders.get(&desc.vertex_shader) {
            if vs_rec.stage == DuctShaderStage::Compute && !desc.fragment_shader.is_valid() {
                let cs_mod = vs_rec.module;
                if let Some(&cached) = self.compute_pipelines.get(&desc.vertex_shader) {
                    self.active_compute_pipeline = cached;
                } else if cs_mod != VK_NULL_HANDLE && self.compute_pipeline_layout != VK_NULL_HANDLE
                {
                    if let Some(dfns) = &self.device_fns {
                        let entry = b"main\0";
                        let cp_info = VkComputePipelineCreateInfo {
                            sType: VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            stage: VkPipelineShaderStageCreateInfo {
                                sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                                pNext: std::ptr::null(),
                                flags: 0,
                                stage: VK_SHADER_STAGE_COMPUTE_BIT,
                                module: cs_mod,
                                pName: entry.as_ptr() as *const c_char,
                                pSpecializationInfo: std::ptr::null(),
                            },
                            layout: self.compute_pipeline_layout,
                            basePipelineHandle: VK_NULL_HANDLE,
                            basePipelineIndex: -1,
                        };
                        let mut cp = VK_NULL_HANDLE;
                        if unsafe {
                            (dfns.CreateComputePipelines)(
                                self.device,
                                VK_NULL_HANDLE,
                                1,
                                &cp_info,
                                std::ptr::null(),
                                &mut cp,
                            )
                        } == VK_SUCCESS
                        {
                            self.compute_pipelines.insert(desc.vertex_shader, cp);
                            self.active_compute_pipeline = cp;
                        }
                    }
                }
                return DuctResult::Success;
            }
        }

        self.current_pso = Some(*desc);
        let is_offscreen = self.active_render_target.is_valid();
        let pipeline = self.get_or_create_graphics_pipeline(desc, is_offscreen);
        self.active_graphics_pipeline = pipeline;

        if self.cmd_recording && self.render_pass_active && pipeline != VK_NULL_HANDLE {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdBindPipeline)(
                        self.command_buffer,
                        VK_PIPELINE_BIND_POINT_GRAPHICS,
                        pipeline,
                    );
                }
            }
        }

        DuctResult::Success
    }

    pub(crate) fn set_wireframe_impl(&mut self, enabled: bool) -> DuctResult {
        self.wireframe = enabled;
        DuctResult::Success
    }

}
