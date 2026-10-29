use crate::types::*;
use crate::types::VkPipeline;
use crate::vk::*;
use duct_core::*;
use std::ffi::c_void;

impl VkPipeline {
    pub(crate) fn create_texture_2d_impl(
        &mut self,
        width: u32,
        height: u32,
        format: DuctTextureFormat,
        _data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctResult> {
        let vk_format = map_texture_format(format);
        let mut image = VK_NULL_HANDLE;
        let mut memory = VK_NULL_HANDLE;
        let mut view = VK_NULL_HANDLE;
        let mut sampler = VK_NULL_HANDLE;

        if let Some(dfns) = &self.device_fns {
            if !self.device.is_null() && width > 0 && height > 0 {
                let img_info = VkImageCreateInfo {
                    sType: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                    imageType: VK_IMAGE_TYPE_2D,
                    format: vk_format,
                    extent: VkExtent3D {
                        width,
                        height,
                        depth: 1,
                    },
                    mipLevels: 1,
                    arrayLayers: 1,
                    samples: VK_SAMPLE_COUNT_1_BIT,
                    tiling: VK_IMAGE_TILING_OPTIMAL,
                    usage: VK_IMAGE_USAGE_SAMPLED_BIT
                        | VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT
                        | VK_IMAGE_USAGE_TRANSFER_DST_BIT
                        | VK_IMAGE_USAGE_TRANSFER_SRC_BIT,
                    sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                    queueFamilyIndexCount: 0,
                    pQueueFamilyIndices: std::ptr::null(),
                    initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
                };

                if unsafe {
                    (dfns.CreateImage)(self.device, &img_info, std::ptr::null(), &mut image)
                } == VK_SUCCESS
                {
                    let mut mem_reqs = VkMemoryRequirements::default();
                    unsafe {
                        (dfns.GetImageMemoryRequirements)(self.device, image, &mut mem_reqs);
                    }

                    if let Some(mem_type) = self
                        .find_memory_type(mem_reqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT)
                        .or_else(|| self.find_memory_type(mem_reqs.memoryTypeBits, 0))
                    {
                        let alloc_info = VkMemoryAllocateInfo {
                            sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                            pNext: std::ptr::null(),
                            allocationSize: mem_reqs.size,
                            memoryTypeIndex: mem_type,
                        };
                        if unsafe {
                            (dfns.AllocateMemory)(
                                self.device,
                                &alloc_info,
                                std::ptr::null(),
                                &mut memory,
                            )
                        } == VK_SUCCESS
                        {
                            unsafe {
                                let _ = (dfns.BindImageMemory)(self.device, image, memory, 0);
                            }

                            let iv_info = VkImageViewCreateInfo {
                                sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                                pNext: std::ptr::null(),
                                flags: 0,
                                image,
                                viewType: VK_IMAGE_VIEW_TYPE_2D,
                                format: vk_format,
                                components: VkComponentMapping::default(),
                                subresourceRange: VkImageSubresourceRange {
                                    aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                    baseMipLevel: 0,
                                    levelCount: 1,
                                    baseArrayLayer: 0,
                                    layerCount: 1,
                                },
                            };
                            let _ = unsafe {
                                (dfns.CreateImageView)(
                                    self.device,
                                    &iv_info,
                                    std::ptr::null(),
                                    &mut view,
                                )
                            };
                        }
                    }

                    let sampler_info = VkSamplerCreateInfo {
                        sType: VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        magFilter: VK_FILTER_LINEAR,
                        minFilter: VK_FILTER_LINEAR,
                        mipmapMode: VK_SAMPLER_MIPMAP_MODE_LINEAR,
                        addressModeU: VK_SAMPLER_ADDRESS_MODE_REPEAT,
                        addressModeV: VK_SAMPLER_ADDRESS_MODE_REPEAT,
                        addressModeW: VK_SAMPLER_ADDRESS_MODE_REPEAT,
                        mipLodBias: 0.0,
                        anisotropyEnable: VK_FALSE,
                        maxAnisotropy: 1.0,
                        compareEnable: VK_FALSE,
                        compareOp: VK_COMPARE_OP_ALWAYS,
                        minLod: 0.0,
                        maxLod: 1.0,
                        borderColor: 0,
                        unnormalizedCoordinates: VK_FALSE,
                    };
                    let _ = unsafe {
                        (dfns.CreateSampler)(
                            self.device,
                            &sampler_info,
                            std::ptr::null(),
                            &mut sampler,
                        )
                    };
                }
            }
        }

        self.allocated_gpu_bytes += (width as usize) * (height as usize) * 4;

        let handle = DuctTextureHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.textures.insert(
            handle,
            VkTextureRecord {
                image,
                memory,
                view,
                sampler,
                width,
                height,
                vk_format,
                min_filter: DuctFilterMode::Linear,
                mag_filter: DuctFilterMode::Linear,
                wrap_s: DuctWrapMode::Repeat,
                wrap_t: DuctWrapMode::Repeat,
            },
        );
        if let Some(initial_bytes) = _data {
            let _ = self.update_texture_2d(handle, 0, 0, width, height, initial_bytes);
        }
        Ok(handle)
    }

    pub(crate) fn update_texture_2d_impl(
        &mut self,
        handle: DuctTextureHandle,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> DuctResult {
        let tex_image = if let Some(tex) = self.textures.get(&handle) {
            if x + width > tex.width || y + height > tex.height {
                return DuctResult::ErrorInvalidParameter;
            }
            tex.image
        } else {
            return DuctResult::ErrorResourceNotFound;
        };

        if tex_image != VK_NULL_HANDLE
            && !self.cmd_recording
            && !data.is_empty()
            && width > 0
            && height > 0
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    let _ = (dfns.DeviceWaitIdle)(self.device);
                    let size = data.len();
                    let buf_info = VkBufferCreateInfo {
                        sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        size: size as VkDeviceSize,
                        usage: VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
                        sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                        queueFamilyIndexCount: 0,
                        pQueueFamilyIndices: std::ptr::null(),
                    };
                    let mut staging_buf = VK_NULL_HANDLE;
                    let mut staging_mem = VK_NULL_HANDLE;
                    if (dfns.CreateBuffer)(
                        self.device,
                        &buf_info,
                        std::ptr::null(),
                        &mut staging_buf,
                    ) == VK_SUCCESS
                    {
                        let mut mem_reqs = VkMemoryRequirements::default();
                        (dfns.GetBufferMemoryRequirements)(self.device, staging_buf, &mut mem_reqs);
                        if let Some(mem_idx) = self.find_memory_type(
                            mem_reqs.memoryTypeBits,
                            VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                                | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                        ) {
                            let alloc_info = VkMemoryAllocateInfo {
                                sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                                pNext: std::ptr::null(),
                                allocationSize: mem_reqs.size,
                                memoryTypeIndex: mem_idx,
                            };
                            if (dfns.AllocateMemory)(
                                self.device,
                                &alloc_info,
                                std::ptr::null(),
                                &mut staging_mem,
                            ) == VK_SUCCESS
                            {
                                let _ = (dfns.BindBufferMemory)(
                                    self.device,
                                    staging_buf,
                                    staging_mem,
                                    0,
                                );
                                let mut mapped: *mut c_void = std::ptr::null_mut();
                                if (dfns.MapMemory)(
                                    self.device,
                                    staging_mem,
                                    0,
                                    size as VkDeviceSize,
                                    0,
                                    &mut mapped,
                                ) == VK_SUCCESS
                                    && !mapped.is_null()
                                {
                                    std::ptr::copy_nonoverlapping(
                                        data.as_ptr(),
                                        mapped as *mut u8,
                                        size,
                                    );
                                    (dfns.UnmapMemory)(self.device, staging_mem);
                                }

                                let _ = (dfns.ResetCommandBuffer)(self.command_buffer, 0);
                                let begin_info = VkCommandBufferBeginInfo {
                                    sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                                    pNext: std::ptr::null(),
                                    flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                                    pInheritanceInfo: std::ptr::null(),
                                };
                                if (dfns.BeginCommandBuffer)(self.command_buffer, &begin_info)
                                    == VK_SUCCESS
                                {
                                    let to_dst = VkImageMemoryBarrier {
                                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                                        pNext: std::ptr::null(),
                                        srcAccessMask: 0,
                                        dstAccessMask: VK_ACCESS_TRANSFER_WRITE_BIT,
                                        oldLayout: VK_IMAGE_LAYOUT_UNDEFINED,
                                        newLayout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                        srcQueueFamilyIndex: !0u32,
                                        dstQueueFamilyIndex: !0u32,
                                        image: tex_image,
                                        subresourceRange: VkImageSubresourceRange {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            baseMipLevel: 0,
                                            levelCount: 1,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                    };
                                    (dfns.CmdPipelineBarrier)(
                                        self.command_buffer,
                                        VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                                        VK_PIPELINE_STAGE_TRANSFER_BIT,
                                        0,
                                        0,
                                        std::ptr::null(),
                                        0,
                                        std::ptr::null(),
                                        1,
                                        &to_dst,
                                    );

                                    let copy_region = VkBufferImageCopy {
                                        bufferOffset: 0,
                                        bufferRowLength: 0,
                                        bufferImageHeight: 0,
                                        imageSubresource: VkImageSubresourceLayers {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            mipLevel: 0,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                        imageOffset: VkOffset3D {
                                            x: x as i32,
                                            y: y as i32,
                                            z: 0,
                                        },
                                        imageExtent: VkExtent3D {
                                            width,
                                            height,
                                            depth: 1,
                                        },
                                    };
                                    (dfns.CmdCopyBufferToImage)(
                                        self.command_buffer,
                                        staging_buf,
                                        tex_image,
                                        VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                        1,
                                        &copy_region,
                                    );

                                    let to_color = VkImageMemoryBarrier {
                                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                                        pNext: std::ptr::null(),
                                        srcAccessMask: VK_ACCESS_TRANSFER_WRITE_BIT,
                                        dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                                        oldLayout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                        newLayout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                        srcQueueFamilyIndex: !0u32,
                                        dstQueueFamilyIndex: !0u32,
                                        image: tex_image,
                                        subresourceRange: VkImageSubresourceRange {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            baseMipLevel: 0,
                                            levelCount: 1,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                    };
                                    (dfns.CmdPipelineBarrier)(
                                        self.command_buffer,
                                        VK_PIPELINE_STAGE_TRANSFER_BIT,
                                        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                                        0,
                                        0,
                                        std::ptr::null(),
                                        0,
                                        std::ptr::null(),
                                        1,
                                        &to_color,
                                    );

                                    let _ = (dfns.EndCommandBuffer)(self.command_buffer);
                                    let submit = VkSubmitInfo {
                                        sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                                        pNext: std::ptr::null(),
                                        waitSemaphoreCount: 0,
                                        pWaitSemaphores: std::ptr::null(),
                                        pWaitDstStageMask: std::ptr::null(),
                                        commandBufferCount: 1,
                                        pCommandBuffers: &self.command_buffer,
                                        signalSemaphoreCount: 0,
                                        pSignalSemaphores: std::ptr::null(),
                                    };
                                    let _ = (dfns.QueueSubmit)(
                                        self.queue,
                                        1,
                                        &submit,
                                        VK_NULL_HANDLE,
                                    );
                                    let _ = (dfns.QueueWaitIdle)(self.queue);
                                }
                                (dfns.FreeMemory)(self.device, staging_mem, std::ptr::null());
                            }
                        }
                        (dfns.DestroyBuffer)(self.device, staging_buf, std::ptr::null());
                    }
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn destroy_texture_impl(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if let Some(t) = self.textures.remove(&handle) {
            self.allocated_gpu_bytes = self
                .allocated_gpu_bytes
                .saturating_sub((t.width as usize) * (t.height as usize) * 4);
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        let _ = (dfns.DeviceWaitIdle)(self.device);
                        if t.sampler != VK_NULL_HANDLE {
                            (dfns.DestroySampler)(self.device, t.sampler, std::ptr::null());
                        }
                        if t.view != VK_NULL_HANDLE {
                            (dfns.DestroyImageView)(self.device, t.view, std::ptr::null());
                        }
                        if t.image != VK_NULL_HANDLE {
                            (dfns.DestroyImage)(self.device, t.image, std::ptr::null());
                        }
                        if t.memory != VK_NULL_HANDLE {
                            (dfns.FreeMemory)(self.device, t.memory, std::ptr::null());
                        }
                    }
                }
            }
            for slot in &mut self.bound_textures {
                if *slot == handle {
                    *slot = DuctTextureHandle::INVALID;
                    self.descriptor_set_dirty = true;
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_texture_impl(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult {
        if (slot as usize) < self.bound_textures.len() {
            if handle.is_valid() {
                if self.textures.contains_key(&handle) {
                    if self.bound_textures[slot as usize] != handle {
                        self.bound_textures[slot as usize] = handle;
                        self.descriptor_set_dirty = true;
                    }
                    DuctResult::Success
                } else {
                    DuctResult::ErrorResourceNotFound
                }
            } else {
                if self.bound_textures[slot as usize] != DuctTextureHandle::INVALID {
                    self.bound_textures[slot as usize] = DuctTextureHandle::INVALID;
                    self.descriptor_set_dirty = true;
                }
                DuctResult::Success
            }
        } else {
            DuctResult::ErrorInvalidParameter
        }
    }

    pub(crate) fn generate_mipmaps_impl(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if self.textures.contains_key(&handle) {
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn set_texture_filter_impl(
        &mut self,
        handle: DuctTextureHandle,
        min: DuctFilterMode,
        mag: DuctFilterMode,
    ) -> DuctResult {
        if let Some(t) = self.textures.get_mut(&handle) {
            t.min_filter = min;
            t.mag_filter = mag;
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        if t.sampler != VK_NULL_HANDLE {
                            (dfns.DestroySampler)(self.device, t.sampler, std::ptr::null());
                        }
                        let sampler_info = VkSamplerCreateInfo {
                            sType: VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            magFilter: map_filter(t.mag_filter),
                            minFilter: map_filter(t.min_filter),
                            mipmapMode: VK_SAMPLER_MIPMAP_MODE_LINEAR,
                            addressModeU: map_address_mode(t.wrap_s),
                            addressModeV: map_address_mode(t.wrap_t),
                            addressModeW: map_address_mode(t.wrap_s),
                            mipLodBias: 0.0,
                            anisotropyEnable: VK_FALSE,
                            maxAnisotropy: 1.0,
                            compareEnable: VK_FALSE,
                            compareOp: VK_COMPARE_OP_ALWAYS,
                            minLod: 0.0,
                            maxLod: 1.0,
                            borderColor: 0,
                            unnormalizedCoordinates: VK_FALSE,
                        };
                        let _ = (dfns.CreateSampler)(
                            self.device,
                            &sampler_info,
                            std::ptr::null(),
                            &mut t.sampler,
                        );
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn set_texture_wrap_impl(
        &mut self,
        handle: DuctTextureHandle,
        wrap_s: DuctWrapMode,
        wrap_t: DuctWrapMode,
    ) -> DuctResult {
        if let Some(t) = self.textures.get_mut(&handle) {
            t.wrap_s = wrap_s;
            t.wrap_t = wrap_t;
            let min = t.min_filter;
            let mag = t.mag_filter;
            self.set_texture_filter(handle, min, mag)
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn create_render_target_impl(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        let color_tex = self.create_texture_2d(desc.width, desc.height, desc.format, None)?;
        let mut framebuffer = VK_NULL_HANDLE;
        let mut depth_image = VK_NULL_HANDLE;
        let mut depth_memory = VK_NULL_HANDLE;
        let mut depth_view = VK_NULL_HANDLE;

        let dfns = match &self.device_fns {
            Some(d) if !self.device.is_null() => *d,
            _ => return Err(DuctResult::ErrorInvalidState),
        };

        let color_view = self.textures.get(&color_tex).map(|t| t.view).unwrap_or(VK_NULL_HANDLE);
        let mut depth_tex = None;
        if color_view != VK_NULL_HANDLE {
                let target_rp = if desc.has_depth && self.offscreen_render_pass_with_depth != VK_NULL_HANDLE {
                    let depth_info = VkImageCreateInfo {
                        sType: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        imageType: VK_IMAGE_TYPE_2D,
                        format: VK_FORMAT_D32_SFLOAT,
                        extent: VkExtent3D {
                            width: desc.width,
                            height: desc.height,
                            depth: 1,
                        },
                        mipLevels: 1,
                        arrayLayers: 1,
                        samples: VK_SAMPLE_COUNT_1_BIT,
                        tiling: VK_IMAGE_TILING_OPTIMAL,
                        usage: VK_IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT | VK_IMAGE_USAGE_SAMPLED_BIT,
                        sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                        queueFamilyIndexCount: 0,
                        pQueueFamilyIndices: std::ptr::null(),
                        initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
                    };

                    if unsafe {
                        (dfns.CreateImage)(self.device, &depth_info, std::ptr::null(), &mut depth_image)
                    } == VK_SUCCESS
                    {
                        let mut mem_reqs = VkMemoryRequirements::default();
                        unsafe {
                            (dfns.GetImageMemoryRequirements)(self.device, depth_image, &mut mem_reqs);
                        }
                        if let Some(mem_type) = self
                            .find_memory_type(mem_reqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT)
                            .or_else(|| self.find_memory_type(mem_reqs.memoryTypeBits, 0))
                        {
                            let alloc_info = VkMemoryAllocateInfo {
                                sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                                pNext: std::ptr::null(),
                                allocationSize: mem_reqs.size,
                                memoryTypeIndex: mem_type,
                            };
                            if unsafe {
                                (dfns.AllocateMemory)(self.device, &alloc_info, std::ptr::null(), &mut depth_memory)
                            } == VK_SUCCESS
                            {
                                unsafe {
                                    let _ = (dfns.BindImageMemory)(self.device, depth_image, depth_memory, 0);
                                }
                                let iv_info = VkImageViewCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                    image: depth_image,
                                    viewType: VK_IMAGE_VIEW_TYPE_2D,
                                    format: VK_FORMAT_D32_SFLOAT,
                                    components: VkComponentMapping::default(),
                                    subresourceRange: VkImageSubresourceRange {
                                        aspectMask: VK_IMAGE_ASPECT_DEPTH_BIT,
                                        baseMipLevel: 0,
                                        levelCount: 1,
                                        baseArrayLayer: 0,
                                        layerCount: 1,
                                    },
                                };
                                unsafe {
                                    let _ = (dfns.CreateImageView)(
                                        self.device,
                                        &iv_info,
                                        std::ptr::null(),
                                        &mut depth_view,
                                    );
                                }

                                let sampler_info = VkSamplerCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                    magFilter: VK_FILTER_LINEAR,
                                    minFilter: VK_FILTER_LINEAR,
                                    mipmapMode: VK_SAMPLER_MIPMAP_MODE_NEAREST,
                                    addressModeU: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                                    addressModeV: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                                    addressModeW: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                                    mipLodBias: 0.0,
                                    anisotropyEnable: VK_FALSE,
                                    maxAnisotropy: 1.0,
                                    compareEnable: VK_FALSE,
                                    compareOp: VK_COMPARE_OP_ALWAYS,
                                    minLod: 0.0,
                                    maxLod: 1.0,
                                    borderColor: 0,
                                    unnormalizedCoordinates: VK_FALSE,
                                };
                                let mut depth_sampler = VK_NULL_HANDLE;
                                unsafe {
                                    let _ = (dfns.CreateSampler)(
                                        self.device,
                                        &sampler_info,
                                        std::ptr::null(),
                                        &mut depth_sampler,
                                    );
                                }
                                let dt_handle = DuctTextureHandle(self.next_handle_id);
                                self.next_handle_id += 1;
                                self.textures.insert(
                                    dt_handle,
                                    VkTextureRecord {
                                        image: depth_image,
                                        memory: depth_memory,
                                        view: depth_view,
                                        sampler: depth_sampler,
                                        width: desc.width,
                                        height: desc.height,
                                        vk_format: VK_FORMAT_D32_SFLOAT,
                                        min_filter: DuctFilterMode::Linear,
                                        mag_filter: DuctFilterMode::Linear,
                                        wrap_s: DuctWrapMode::ClampToEdge,
                                        wrap_t: DuctWrapMode::ClampToEdge,
                                    },
                                );
                                depth_tex = Some(dt_handle);
                            }
                        }
                    }
                    self.offscreen_render_pass_with_depth
                } else {
                    self.offscreen_render_pass
                };

                let mut attachments = Vec::new();
                attachments.push(color_view);
                if desc.has_depth && depth_view != VK_NULL_HANDLE {
                    attachments.push(depth_view);
                }

                let fb_info = VkFramebufferCreateInfo {
                    sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                    renderPass: target_rp,
                    attachmentCount: attachments.len() as u32,
                    pAttachments: attachments.as_ptr(),
                    width: desc.width,
                    height: desc.height,
                    layers: 1,
                };
                let _ = unsafe {
                    (dfns.CreateFramebuffer)(
                        self.device,
                        &fb_info,
                        std::ptr::null(),
                        &mut framebuffer,
                    )
                };
            }

        let handle = DuctRenderTargetHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.render_targets.insert(
            handle,
            VkRenderTargetRecord {
                width: desc.width,
                height: desc.height,
                color_tex,
                depth_tex,
                depth_image,
                depth_memory,
                depth_view,
                framebuffer,
                has_depth: desc.has_depth,
            },
        );
        Ok(handle)
    }

    pub(crate) fn destroy_render_target_impl(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if let Some(rt) = self.render_targets.remove(&handle) {
            if self.active_render_target == handle {
                self.active_render_target = DuctRenderTargetHandle::INVALID;
            }
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        let _ = (dfns.DeviceWaitIdle)(self.device);
                        if rt.framebuffer != VK_NULL_HANDLE {
                            (dfns.DestroyFramebuffer)(self.device, rt.framebuffer, std::ptr::null());
                        }
                        if rt.depth_tex.is_none() {
                            if rt.depth_view != VK_NULL_HANDLE {
                                (dfns.DestroyImageView)(self.device, rt.depth_view, std::ptr::null());
                            }
                            if rt.depth_image != VK_NULL_HANDLE {
                                (dfns.DestroyImage)(self.device, rt.depth_image, std::ptr::null());
                            }
                            if rt.depth_memory != VK_NULL_HANDLE {
                                (dfns.FreeMemory)(self.device, rt.depth_memory, std::ptr::null());
                            }
                        }
                    }
                }
            }
            if let Some(dt) = rt.depth_tex {
                let _ = self.destroy_texture(dt);
            }
            let _ = self.destroy_texture(rt.color_tex);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_render_target_impl(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if !handle.is_valid() {
            self.active_render_target = DuctRenderTargetHandle::INVALID;
            self.current_pass_is_offscreen = false;
        } else if self.render_targets.contains_key(&handle) {
            self.active_render_target = handle;
            self.current_pass_is_offscreen = true;
        } else {
            return DuctResult::ErrorResourceNotFound;
        }

        // If recording inside active command buffer, seamlessly transition render passes!
        if self.cmd_recording {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    if self.render_pass_active {
                        (dfns.CmdEndRenderPass)(self.command_buffer);
                        self.render_pass_active = false;
                    }

                    let mut target_fb = VK_NULL_HANDLE;
                    let mut target_rp = VK_NULL_HANDLE;
                    let mut target_extent = VkExtent2D {
                        width: self.viewport_width,
                        height: self.viewport_height,
                    };
                    let mut has_depth = true;

                    if self.current_pass_is_offscreen {
                        if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                            target_fb = rt.framebuffer;
                            target_rp = if rt.has_depth && self.offscreen_render_pass_with_depth != VK_NULL_HANDLE {
                                self.offscreen_render_pass_with_depth
                            } else {
                                self.offscreen_render_pass
                            };
                            target_extent = VkExtent2D {
                                width: rt.width,
                                height: rt.height,
                            };
                            has_depth = rt.has_depth;
                        }
                    } else if self.swapchain_active && self.swapchain_image_acquired {
                        if (self.current_image_index as usize) < self.framebuffers.len() {
                            target_fb = self.framebuffers[self.current_image_index as usize];
                            target_rp = self.render_pass;
                            target_extent = self.swapchain_extent;
                            has_depth = self.depth_image_view != VK_NULL_HANDLE;
                        }
                    }

                    if target_rp != VK_NULL_HANDLE && target_fb != VK_NULL_HANDLE {
                        let vp = VkViewport {
                            x: 0.0,
                            y: target_extent.height as f32,
                            width: target_extent.width as f32,
                            height: -(target_extent.height as f32),
                            minDepth: 0.0,
                            maxDepth: 1.0,
                        };
                        (dfns.CmdSetViewport)(self.command_buffer, 0, 1, &vp);

                        let scissor = VkRect2D {
                            offset: VkOffset2D { x: 0, y: 0 },
                            extent: target_extent,
                        };
                        (dfns.CmdSetScissor)(self.command_buffer, 0, 1, &scissor);

                        let clear_vals = [
                            VkClearValue {
                                float32: [
                                    self.last_clear_color.r,
                                    self.last_clear_color.g,
                                    self.last_clear_color.b,
                                    self.last_clear_color.a,
                                ],
                            },
                            VkClearValue {
                                float32: [1.0, 0.0, 0.0, 0.0],
                            },
                        ];
                        let clear_count = if has_depth { 2 } else { 1 };
                        let rp_begin = VkRenderPassBeginInfo {
                            sType: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
                            pNext: std::ptr::null(),
                            renderPass: target_rp,
                            framebuffer: target_fb,
                            renderArea: VkRect2D {
                                offset: VkOffset2D { x: 0, y: 0 },
                                extent: target_extent,
                            },
                            clearValueCount: clear_count,
                            pClearValues: clear_vals.as_ptr(),
                        };
                        (dfns.CmdBeginRenderPass)(
                            self.command_buffer,
                            &rp_begin,
                            VK_SUBPASS_CONTENTS_INLINE,
                        );
                        self.render_pass_active = true;
                    }
                }
            }
        }

        DuctResult::Success
    }

    pub(crate) fn get_render_target_texture_impl(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        if let Some(rt) = self.render_targets.get(&handle) {
            Ok(rt.color_tex)
        } else {
            Err(DuctResult::ErrorResourceNotFound)
        }
    }

    pub(crate) fn get_render_target_depth_texture_impl(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        if let Some(rt) = self.render_targets.get(&handle) {
            if let Some(dt) = rt.depth_tex {
                Ok(dt)
            } else {
                Err(DuctResult::ErrorUnsupported)
            }
        } else {
            Err(DuctResult::ErrorResourceNotFound)
        }
    }

    pub(crate) fn blit_render_target_impl(
        &mut self,
        src: DuctRenderTargetHandle,
        dst: DuctRenderTargetHandle,
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        filter: DuctFilterMode,
    ) -> DuctResult {
        let src_img = if src.is_valid() {
            self.render_targets
                .get(&src)
                .and_then(|rt| self.textures.get(&rt.color_tex))
                .map(|t| t.image)
                .unwrap_or(VK_NULL_HANDLE)
        } else {
            VK_NULL_HANDLE
        };

        let is_dst_swapchain = !dst.is_valid();
        if is_dst_swapchain && self.swapchain_active && !self.swapchain_image_acquired {
            if let Some(dfns) = &self.device_fns {
                if let Some(acquire_fn) = dfns.AcquireNextImageKHR {
                    let mut img_idx = 0u32;
                    let acq_res = unsafe {
                        acquire_fn(
                            self.device,
                            self.swapchain,
                            u64::MAX,
                            self.image_available_sem,
                            VK_NULL_HANDLE,
                            &mut img_idx,
                        )
                    };
                    if acq_res == VK_SUCCESS || acq_res == VK_SUBOPTIMAL_KHR {
                        self.current_image_index = img_idx;
                        self.swapchain_image_acquired = true;
                    }
                }
            }
        }

        let dst_img = if is_dst_swapchain {
            if self.swapchain_active && (self.current_image_index as usize) < self.swapchain_images.len() {
                self.swapchain_images[self.current_image_index as usize]
            } else {
                VK_NULL_HANDLE
            }
        } else {
            self.render_targets
                .get(&dst)
                .and_then(|rt| self.textures.get(&rt.color_tex))
                .map(|t| t.image)
                .unwrap_or(VK_NULL_HANDLE)
        };

        if src_img == VK_NULL_HANDLE
            || dst_img == VK_NULL_HANDLE
            || src_w == 0
            || src_h == 0
            || dst_w == 0
            || dst_h == 0
        {
            return DuctResult::ErrorInvalidParameter;
        }

        if let Some(dfns) = &self.device_fns {
            unsafe {
                if self.cmd_recording && self.render_pass_active {
                    (dfns.CmdEndRenderPass)(self.command_buffer);
                    self.render_pass_active = false;
                }

                let one_time = !self.cmd_recording;
                if one_time {
                    let _ = (dfns.DeviceWaitIdle)(self.device);
                    let _ = (dfns.ResetCommandBuffer)(self.command_buffer, 0);
                    let begin_info = VkCommandBufferBeginInfo {
                        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                        pNext: std::ptr::null(),
                        flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                        pInheritanceInfo: std::ptr::null(),
                    };
                    let _ = (dfns.BeginCommandBuffer)(self.command_buffer, &begin_info);
                }

                let barriers_in = [
                    VkImageMemoryBarrier {
                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                        pNext: std::ptr::null(),
                        srcAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        dstAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                        oldLayout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                        newLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                        srcQueueFamilyIndex: !0u32,
                        dstQueueFamilyIndex: !0u32,
                        image: src_img,
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    },
                    VkImageMemoryBarrier {
                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                        pNext: std::ptr::null(),
                        srcAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        dstAccessMask: VK_ACCESS_TRANSFER_WRITE_BIT,
                        oldLayout: if is_dst_swapchain {
                            VK_IMAGE_LAYOUT_UNDEFINED
                        } else {
                            VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL
                        },
                        newLayout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                        srcQueueFamilyIndex: !0u32,
                        dstQueueFamilyIndex: !0u32,
                        image: dst_img,
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    },
                ];
                (dfns.CmdPipelineBarrier)(
                    self.command_buffer,
                    VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                    VK_PIPELINE_STAGE_TRANSFER_BIT,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    2,
                    barriers_in.as_ptr(),
                );

                let blit_region = VkImageBlit {
                    srcSubresource: VkImageSubresourceLayers {
                        aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                        mipLevel: 0,
                        baseArrayLayer: 0,
                        layerCount: 1,
                    },
                    srcOffsets: [
                        VkOffset3D { x: 0, y: 0, z: 0 },
                        VkOffset3D {
                            x: src_w as i32,
                            y: src_h as i32,
                            z: 1,
                        },
                    ],
                    dstSubresource: VkImageSubresourceLayers {
                        aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                        mipLevel: 0,
                        baseArrayLayer: 0,
                        layerCount: 1,
                    },
                    dstOffsets: [
                        VkOffset3D { x: 0, y: 0, z: 0 },
                        VkOffset3D {
                            x: dst_w as i32,
                            y: dst_h as i32,
                            z: 1,
                        },
                    ],
                };
                (dfns.CmdBlitImage)(
                    self.command_buffer,
                    src_img,
                    VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                    dst_img,
                    VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    1,
                    &blit_region,
                    map_filter(filter),
                );

                let dst_final_layout = if is_dst_swapchain {
                    VK_IMAGE_LAYOUT_PRESENT_SRC_KHR
                } else {
                    VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL
                };

                let barriers_out = [
                    VkImageMemoryBarrier {
                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                        pNext: std::ptr::null(),
                        srcAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                        dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        oldLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                        newLayout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                        srcQueueFamilyIndex: !0u32,
                        dstQueueFamilyIndex: !0u32,
                        image: src_img,
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    },
                    VkImageMemoryBarrier {
                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                        pNext: std::ptr::null(),
                        srcAccessMask: VK_ACCESS_TRANSFER_WRITE_BIT,
                        dstAccessMask: VK_ACCESS_MEMORY_READ_BIT,
                        oldLayout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                        newLayout: dst_final_layout,
                        srcQueueFamilyIndex: !0u32,
                        dstQueueFamilyIndex: !0u32,
                        image: dst_img,
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    },
                ];
                (dfns.CmdPipelineBarrier)(
                    self.command_buffer,
                    VK_PIPELINE_STAGE_TRANSFER_BIT,
                    VK_PIPELINE_STAGE_ALL_COMMANDS_BIT,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    2,
                    barriers_out.as_ptr(),
                );

                if one_time {
                    let _ = (dfns.EndCommandBuffer)(self.command_buffer);
                    let wait_stage = VK_PIPELINE_STAGE_TRANSFER_BIT;
                    let use_swapchain_sems = is_dst_swapchain
                        && self.swapchain_active
                        && self.swapchain_image_acquired
                        && self.image_available_sem != VK_NULL_HANDLE
                        && self.render_finished_sem != VK_NULL_HANDLE;

                    let submit = if use_swapchain_sems {
                        VkSubmitInfo {
                            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                            pNext: std::ptr::null(),
                            waitSemaphoreCount: 1,
                            pWaitSemaphores: &self.image_available_sem,
                            pWaitDstStageMask: &wait_stage,
                            commandBufferCount: 1,
                            pCommandBuffers: &self.command_buffer,
                            signalSemaphoreCount: 1,
                            pSignalSemaphores: &self.render_finished_sem,
                        }
                    } else {
                        VkSubmitInfo {
                            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                            pNext: std::ptr::null(),
                            waitSemaphoreCount: 0,
                            pWaitSemaphores: std::ptr::null(),
                            pWaitDstStageMask: std::ptr::null(),
                            commandBufferCount: 1,
                            pCommandBuffers: &self.command_buffer,
                            signalSemaphoreCount: 0,
                            pSignalSemaphores: std::ptr::null(),
                        }
                    };
                    let _ = (dfns.QueueSubmit)(self.queue, 1, &submit, VK_NULL_HANDLE);
                    let _ = (dfns.QueueWaitIdle)(self.queue);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn read_pixels_impl(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        out_rgba: &mut [u8],
    ) -> DuctResult {
        let req = (width as usize) * (height as usize) * 4;
        if out_rgba.len() < req || width == 0 || height == 0 {
            return DuctResult::ErrorInvalidParameter;
        }

        // Determine source image to read back from (offscreen FBO or swapchain image)
        let (src_img, src_fmt, img_w, img_h, current_layout) =
            if self.active_render_target.is_valid() {
                if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                    if let Some(tex) = self.textures.get(&rt.color_tex) {
                        (
                            tex.image,
                            tex.vk_format,
                            rt.width,
                            rt.height,
                            VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                        )
                    } else {
                        (VK_NULL_HANDLE, 0, 0, 0, 0)
                    }
                } else {
                    (VK_NULL_HANDLE, 0, 0, 0, 0)
                }
            } else if self.swapchain_active
                && (self.current_image_index as usize) < self.swapchain_images.len()
            {
                (
                    self.swapchain_images[self.current_image_index as usize],
                    self.swapchain_format,
                    self.swapchain_extent.width,
                    self.swapchain_extent.height,
                    VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
                )
            } else {
                (VK_NULL_HANDLE, 0, 0, 0, 0)
            };

        if src_img != VK_NULL_HANDLE
            && !self.cmd_recording
            && x + width <= img_w
            && y + height <= img_h
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    let _ = (dfns.DeviceWaitIdle)(self.device);

                    let buf_info = VkBufferCreateInfo {
                        sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        size: req as VkDeviceSize,
                        usage: VK_BUFFER_USAGE_TRANSFER_DST_BIT,
                        sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                        queueFamilyIndexCount: 0,
                        pQueueFamilyIndices: std::ptr::null(),
                    };
                    let mut staging_buf = VK_NULL_HANDLE;
                    let mut staging_mem = VK_NULL_HANDLE;

                    if (dfns.CreateBuffer)(
                        self.device,
                        &buf_info,
                        std::ptr::null(),
                        &mut staging_buf,
                    ) == VK_SUCCESS
                    {
                        let mut mem_reqs = VkMemoryRequirements::default();
                        (dfns.GetBufferMemoryRequirements)(self.device, staging_buf, &mut mem_reqs);
                        if let Some(mem_idx) = self.find_memory_type(
                            mem_reqs.memoryTypeBits,
                            VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                                | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                        ) {
                            let alloc_info = VkMemoryAllocateInfo {
                                sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                                pNext: std::ptr::null(),
                                allocationSize: mem_reqs.size,
                                memoryTypeIndex: mem_idx,
                            };
                            if (dfns.AllocateMemory)(
                                self.device,
                                &alloc_info,
                                std::ptr::null(),
                                &mut staging_mem,
                            ) == VK_SUCCESS
                            {
                                let _ = (dfns.BindBufferMemory)(
                                    self.device,
                                    staging_buf,
                                    staging_mem,
                                    0,
                                );

                                let _ = (dfns.ResetCommandBuffer)(self.command_buffer, 0);
                                let begin_info = VkCommandBufferBeginInfo {
                                    sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                                    pNext: std::ptr::null(),
                                    flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                                    pInheritanceInfo: std::ptr::null(),
                                };
                                if (dfns.BeginCommandBuffer)(self.command_buffer, &begin_info)
                                    == VK_SUCCESS
                                {
                                    let to_src = VkImageMemoryBarrier {
                                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                                        pNext: std::ptr::null(),
                                        srcAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                                        dstAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                                        oldLayout: current_layout,
                                        newLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                        srcQueueFamilyIndex: !0u32,
                                        dstQueueFamilyIndex: !0u32,
                                        image: src_img,
                                        subresourceRange: VkImageSubresourceRange {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            baseMipLevel: 0,
                                            levelCount: 1,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                    };
                                    (dfns.CmdPipelineBarrier)(
                                        self.command_buffer,
                                        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                                        VK_PIPELINE_STAGE_TRANSFER_BIT,
                                        0,
                                        0,
                                        std::ptr::null(),
                                        0,
                                        std::ptr::null(),
                                        1,
                                        &to_src,
                                    );

                                    let copy_region = VkBufferImageCopy {
                                        bufferOffset: 0,
                                        bufferRowLength: 0,
                                        bufferImageHeight: 0,
                                        imageSubresource: VkImageSubresourceLayers {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            mipLevel: 0,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                        imageOffset: VkOffset3D {
                                            x: x as i32,
                                            y: y as i32,
                                            z: 0,
                                        },
                                        imageExtent: VkExtent3D {
                                            width,
                                            height,
                                            depth: 1,
                                        },
                                    };
                                    (dfns.CmdCopyImageToBuffer)(
                                        self.command_buffer,
                                        src_img,
                                        VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                        staging_buf,
                                        1,
                                        &copy_region,
                                    );

                                    let restore = VkImageMemoryBarrier {
                                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                                        pNext: std::ptr::null(),
                                        srcAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                                        dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                                        oldLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                        newLayout: current_layout,
                                        srcQueueFamilyIndex: !0u32,
                                        dstQueueFamilyIndex: !0u32,
                                        image: src_img,
                                        subresourceRange: VkImageSubresourceRange {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            baseMipLevel: 0,
                                            levelCount: 1,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                    };
                                    (dfns.CmdPipelineBarrier)(
                                        self.command_buffer,
                                        VK_PIPELINE_STAGE_TRANSFER_BIT,
                                        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                                        0,
                                        0,
                                        std::ptr::null(),
                                        0,
                                        std::ptr::null(),
                                        1,
                                        &restore,
                                    );

                                    let _ = (dfns.EndCommandBuffer)(self.command_buffer);
                                    let submit = VkSubmitInfo {
                                        sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                                        pNext: std::ptr::null(),
                                        waitSemaphoreCount: 0,
                                        pWaitSemaphores: std::ptr::null(),
                                        pWaitDstStageMask: std::ptr::null(),
                                        commandBufferCount: 1,
                                        pCommandBuffers: &self.command_buffer,
                                        signalSemaphoreCount: 0,
                                        pSignalSemaphores: std::ptr::null(),
                                    };
                                    let _ = (dfns.QueueSubmit)(
                                        self.queue,
                                        1,
                                        &submit,
                                        VK_NULL_HANDLE,
                                    );
                                    let _ = (dfns.QueueWaitIdle)(self.queue);

                                    let mut mapped: *mut c_void = std::ptr::null_mut();
                                    if (dfns.MapMemory)(
                                        self.device,
                                        staging_mem,
                                        0,
                                        req as VkDeviceSize,
                                        0,
                                        &mut mapped,
                                    ) == VK_SUCCESS
                                        && !mapped.is_null()
                                    {
                                        std::ptr::copy_nonoverlapping(
                                            mapped as *const u8,
                                            out_rgba.as_mut_ptr(),
                                            req,
                                        );
                                        (dfns.UnmapMemory)(self.device, staging_mem);

                                        if src_fmt == VK_FORMAT_B8G8R8A8_UNORM
                                            || src_fmt == VK_FORMAT_B8G8R8A8_SRGB
                                        {
                                            for px in out_rgba[..req].chunks_exact_mut(4) {
                                                px.swap(0, 2);
                                            }
                                        }
                                    }
                                }
                                (dfns.FreeMemory)(self.device, staging_mem, std::ptr::null());
                            }
                        }
                        (dfns.DestroyBuffer)(self.device, staging_buf, std::ptr::null());
                        return DuctResult::Success;
                    }
                }
            }
        }

        out_rgba[..req].fill(255);
        DuctResult::Success
    }

}
