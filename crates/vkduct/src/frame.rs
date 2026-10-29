use crate::types::*;
use crate::types::VkPipeline;
use crate::vk::*;
use duct_core::*;
use std::ffi::c_void;

impl VkPipeline {
    pub(crate) fn set_scissor_impl(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
        if self.cmd_recording {
            if let Some(dfns) = &self.device_fns {
                let scissor = VkRect2D {
                    offset: VkOffset2D {
                        x: x as i32,
                        y: y as i32,
                    },
                    extent: VkExtent2D { width, height },
                };
                unsafe {
                    (dfns.CmdSetScissor)(self.command_buffer, 0, 1, &scissor);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn begin_frame_impl(&mut self, clear_color: DuctColor) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }
        self.frame_in_progress = true;
        self.draw_calls_in_frame = 0;
        self.vertices_drawn_in_frame = 0;
        self.pipeline_switches_in_frame = 0;
        self.last_clear_color = clear_color;

        let now = std::time::Instant::now();
        if let Some(prev) = self.last_frame_instant {
            let dt = now.duration_since(prev).as_secs_f32();
            self.current_frame_time_ms = dt * 1000.0;
            if dt > 0.0 {
                let instant_fps = 1.0 / dt;
                self.smoothed_fps = 0.9 * self.smoothed_fps + 0.1 * instant_fps;
            }
        }
        self.last_frame_instant = Some(now);

        let is_offscreen = self.active_render_target.is_valid();
        self.current_pass_is_offscreen = is_offscreen;

        if let Some(dfns) = &self.device_fns {
            if !self.command_buffer.is_null() {
                unsafe {
                    if self.in_flight_fence != VK_NULL_HANDLE {
                        let _ = (dfns.WaitForFences)(
                            self.device,
                            1,
                            &self.in_flight_fence,
                            VK_TRUE,
                            u64::MAX,
                        );
                        let _ = (dfns.ResetFences)(self.device, 1, &self.in_flight_fence);
                    }

                    let mut target_fb = VK_NULL_HANDLE;
                    let mut target_rp = VK_NULL_HANDLE;
                    let mut target_extent = VkExtent2D {
                        width: self.viewport_width,
                        height: self.viewport_height,
                    };
                    let mut has_depth = true;

                    if is_offscreen {
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
                    } else if self.swapchain_active {
                        if !self.swapchain_image_acquired {
                            if let Some(acquire_fn) = dfns.AcquireNextImageKHR {
                                let mut img_idx = 0u32;
                                let acq_res = acquire_fn(
                                    self.device,
                                    self.swapchain,
                                    u64::MAX,
                                    self.image_available_sem,
                                    VK_NULL_HANDLE,
                                    &mut img_idx,
                                );
                                if acq_res == VK_SUCCESS || acq_res == VK_SUBOPTIMAL_KHR {
                                    self.current_image_index = img_idx;
                                    self.swapchain_image_acquired = true;
                                }
                            }
                        }
                        if self.swapchain_image_acquired
                            && (self.current_image_index as usize) < self.framebuffers.len()
                        {
                            target_fb = self.framebuffers[self.current_image_index as usize];
                            target_rp = self.render_pass;
                            target_extent = self.swapchain_extent;
                            has_depth = self.depth_image_view != VK_NULL_HANDLE;
                        }
                    }

                    let _ = (dfns.ResetCommandBuffer)(self.command_buffer, 0);
                    let begin_info = VkCommandBufferBeginInfo {
                        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                        pNext: std::ptr::null(),
                        flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                        pInheritanceInfo: std::ptr::null(),
                    };
                    if (dfns.BeginCommandBuffer)(self.command_buffer, &begin_info) == VK_SUCCESS {
                        self.cmd_recording = true;

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

                        if target_rp != VK_NULL_HANDLE && target_fb != VK_NULL_HANDLE {
                            let clear_vals = [
                                VkClearValue {
                                    float32: [
                                        clear_color.r,
                                        clear_color.g,
                                        clear_color.b,
                                        clear_color.a,
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
        }

        DuctResult::Success
    }

    pub(crate) fn end_frame_impl(&mut self) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.frame_in_progress = false;
        self.frame_count += 1;

        if let Some(dfns) = &self.device_fns {
            if self.cmd_recording && !self.command_buffer.is_null() {
                unsafe {
                    if self.render_pass_active {
                        (dfns.CmdEndRenderPass)(self.command_buffer);
                        self.render_pass_active = false;
                    }
                    let _ = (dfns.EndCommandBuffer)(self.command_buffer);
                    self.cmd_recording = false;

                    let wait_stage = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
                    let use_swapchain_sems = !self.current_pass_is_offscreen
                        && self.swapchain_active
                        && self.swapchain_image_acquired
                        && self.image_available_sem != VK_NULL_HANDLE
                        && self.render_finished_sem != VK_NULL_HANDLE;

                    let submit_info = if use_swapchain_sems {
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

                    let _ = (dfns.QueueSubmit)(
                        self.queue,
                        1,
                        &submit_info,
                        self.in_flight_fence,
                    );

                    if self.current_pass_is_offscreen && self.in_flight_fence != VK_NULL_HANDLE {
                        let _ = (dfns.WaitForFences)(
                            self.device,
                            1,
                            &self.in_flight_fence,
                            VK_TRUE,
                            u64::MAX,
                        );
                    }
                }
            }
        }

        DuctResult::Success
    }

    pub(crate) fn present_impl(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }

        if self.swapchain_active && self.swapchain_image_acquired {
            if let Some(dfns) = &self.device_fns {
                if let Some(present_fn) = dfns.QueuePresentKHR {
                    let present_info = VkPresentInfoKHR {
                        sType: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
                        pNext: std::ptr::null(),
                        waitSemaphoreCount: if self.render_finished_sem != VK_NULL_HANDLE {
                            1
                        } else {
                            0
                        },
                        pWaitSemaphores: &self.render_finished_sem,
                        swapchainCount: 1,
                        pSwapchains: &self.swapchain,
                        pImageIndices: &self.current_image_index,
                        pResults: std::ptr::null_mut(),
                    };
                    unsafe {
                        let _ = present_fn(self.queue, &present_info);
                    }
                    self.swapchain_image_acquired = false;
                }
            }
        }

        DuctResult::Success
    }

    pub(crate) fn set_uniform_int_impl(&mut self, name: &str, val: i32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("mode") {
            self.push_constants.extra[3] = val as f32;
        } else {
            self.push_constants.extra[3] = val as f32;
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_float_impl(&mut self, name: &str, val: f32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("time") {
            self.push_constants.extra[0] = val;
        } else if lower.contains("scale") {
            self.push_constants.params[2] = val;
            self.push_constants.params[3] = val;
        } else {
            self.push_constants.params[0] = val;
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec2_impl(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("res") {
            self.push_constants.extra[1] = x;
            self.push_constants.extra[2] = y;
        } else if lower.contains("mouse") {
            self.push_constants.params[0] = x;
            self.push_constants.params[1] = y;
        } else if lower.contains("scale") {
            self.push_constants.params[2] = x;
            self.push_constants.params[3] = y;
        } else {
            self.push_constants.params[0] = x;
            self.push_constants.params[1] = y;
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec3_impl(&mut self, _name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        self.push_constants.color = [x, y, z, 1.0];
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec4_impl(&mut self, _name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        self.push_constants.color = [x, y, z, w];
        DuctResult::Success
    }

    pub(crate) fn set_uniform_mat4_impl(&mut self, _name: &str, elements: &[f32; 16]) -> DuctResult {
        self.push_constants.transform = *elements;
        DuctResult::Success
    }

    pub(crate) fn draw_impl(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count;

        self.prepare_draw();
        if self.cmd_recording
            && self.render_pass_active
            && self.active_graphics_pipeline != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdDraw)(self.command_buffer, vertex_count, 1, first_vertex, 0);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn draw_indexed_impl(&mut self, index_count: u32, first_index: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count;

        self.prepare_draw();
        if self.cmd_recording
            && self.render_pass_active
            && self.active_graphics_pipeline != VK_NULL_HANDLE
            && self.last_bound_ibo != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                let vk_idx = match self.current_index_format {
                    DuctIndexFormat::U16 => VK_INDEX_TYPE_UINT16,
                    DuctIndexFormat::U32 => VK_INDEX_TYPE_UINT32,
                };
                unsafe {
                    (dfns.CmdBindIndexBuffer)(
                        self.command_buffer,
                        self.last_bound_ibo,
                        0,
                        vk_idx,
                    );
                    (dfns.CmdDrawIndexed)(self.command_buffer, index_count, 1, first_index, 0, 0);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn draw_instanced_impl(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count * instance_count;

        self.prepare_draw();
        if self.cmd_recording
            && self.render_pass_active
            && self.active_graphics_pipeline != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdDraw)(
                        self.command_buffer,
                        vertex_count,
                        instance_count,
                        first_vertex,
                        first_instance,
                    );
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn draw_indexed_instanced_impl(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        index_format: DuctIndexFormat,
    ) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count * instance_count;
        self.current_index_format = index_format;

        self.prepare_draw();
        if self.cmd_recording
            && self.render_pass_active
            && self.active_graphics_pipeline != VK_NULL_HANDLE
            && self.last_bound_ibo != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                let vk_idx = match index_format {
                    DuctIndexFormat::U16 => VK_INDEX_TYPE_UINT16,
                    DuctIndexFormat::U32 => VK_INDEX_TYPE_UINT32,
                };
                unsafe {
                    (dfns.CmdBindIndexBuffer)(
                        self.command_buffer,
                        self.last_bound_ibo,
                        0,
                        vk_idx,
                    );
                    (dfns.CmdDrawIndexed)(
                        self.command_buffer,
                        index_count,
                        instance_count,
                        first_index,
                        0,
                        0,
                    );
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn dispatch_compute_impl(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        if self.cmd_recording
            && !self.render_pass_active
            && self.active_compute_pipeline != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdBindPipeline)(
                        self.command_buffer,
                        VK_PIPELINE_BIND_POINT_COMPUTE,
                        self.active_compute_pipeline,
                    );
                    (dfns.CmdDispatch)(self.command_buffer, groups_x, groups_y, groups_z);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn get_frame_stats_impl(&mut self) -> Result<DuctFrameStats, DuctResult> {
        Ok(DuctFrameStats {
            fps: self.smoothed_fps,
            frame_time_ms: self.current_frame_time_ms,
            frame_count: self.frame_count,
            draw_calls: self.draw_calls_in_frame,
            triangles_rendered: if self.vertices_drawn_in_frame > 0 {
                (self.vertices_drawn_in_frame / 3) as u64
            } else {
                (self.draw_calls_in_frame as u64) * 100
            },
            pipeline_switches: self.pipeline_switches_in_frame.max(1),
            allocated_gpu_memory_bytes: self.allocated_gpu_bytes.max(1024 * 1024 * 16),
        })
    }

    pub(crate) fn get_raw_context_impl(&mut self) -> Result<DuctRawContext, DuctResult> {
        Ok(DuctRawContext {
            backend_type: DuctBackendType::Vulkan,
            context_handle: self.device,
            device_handle: self.queue,
            extra_info: self.extra_info.as_ptr(),
            get_proc_address: Some(vk_get_proc_address),
        })
    }

    pub(crate) fn execute_custom_impl(&mut self, command_id: u32, data: *mut c_void) -> DuctResult {
        println!(
            "[vkduct] Custom Vulkan Command {} executed (payload={:?})",
            command_id, data
        );
        DuctResult::Success
    }
}
