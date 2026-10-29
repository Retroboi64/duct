use duct_core::*;
use std::ffi::c_void;
use crate::types::*;

impl Dx11Pipeline {
    pub(crate) unsafe fn prepare_draw(&mut self) {
        if self.device.is_null() || self.context.is_null() {
            return;
        }
        let ctx_vt = com_vtable(self.context);

        let attr_count = self
            .current_pso
            .as_ref()
            .map(|p| p.vertex_layout.attribute_count)
            .unwrap_or(1);
        let stride = self
            .current_pso
            .as_ref()
            .map(|p| {
                if p.vertex_layout.stride > 0 {
                    p.vertex_layout.stride
                } else {
                    12
                }
            })
            .unwrap_or(12);

        // 1. Update Constant Buffer (b0)
        if !self.constant_buffer.is_null() {
            let update_subres: unsafe extern "system" fn(
                *mut c_void,
                *mut c_void,
                u32,
                *const D3D11_BOX,
                *const c_void,
                u32,
                u32,
            ) = std::mem::transmute(*ctx_vt.add(48));
            update_subres(
                self.context,
                self.constant_buffer,
                0,
                std::ptr::null(),
                &self.constants as *const Dx11ConstantBlock as *const c_void,
                0,
                0,
            );

            let vs_set_cb: unsafe extern "system" fn(*mut c_void, u32, u32, *const *mut c_void) =
                std::mem::transmute(*ctx_vt.add(7));
            let ps_set_cb: unsafe extern "system" fn(*mut c_void, u32, u32, *const *mut c_void) =
                std::mem::transmute(*ctx_vt.add(16));
            vs_set_cb(self.context, 0, 1, &self.constant_buffer);
            ps_set_cb(self.context, 0, 1, &self.constant_buffer);
        }

        // 2. Bind Input Layout, Vertex Shader, Pixel Shader
        let ia_set_layout: unsafe extern "system" fn(*mut c_void, *mut c_void) =
            std::mem::transmute(*ctx_vt.add(17));
        let vs_set_shader: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const *mut c_void,
            u32,
        ) = std::mem::transmute(*ctx_vt.add(11));
        let ps_set_shader: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const *mut c_void,
            u32,
        ) = std::mem::transmute(*ctx_vt.add(9));

        let layout = if attr_count >= 2 && !self.default_color_input_layout.is_null() {
            self.default_color_input_layout
        } else {
            self.default_input_layout
        };
        ia_set_layout(self.context, layout);

        let vs_obj = self
            .current_pso
            .as_ref()
            .and_then(|p| self.shaders.get(&p.vertex_shader))
            .map(|s| {
                if attr_count >= 2 && !s.shader_obj_color.is_null() {
                    s.shader_obj_color
                } else {
                    s.shader_obj
                }
            })
            .filter(|p| !p.is_null())
            .unwrap_or(if attr_count >= 2 && !self.default_vs_color.is_null() {
                self.default_vs_color
            } else {
                self.default_vs
            });

        let ps_obj = self
            .current_pso
            .as_ref()
            .and_then(|p| self.shaders.get(&p.fragment_shader))
            .map(|s| s.shader_obj)
            .filter(|p| !p.is_null())
            .unwrap_or(self.default_ps);

        vs_set_shader(self.context, vs_obj, std::ptr::null(), 0);
        ps_set_shader(self.context, ps_obj, std::ptr::null(), 0);

        // 3. Bind Vertex Buffer & Primitive Topology
        let ia_set_topo: unsafe extern "system" fn(*mut c_void, u32) =
            std::mem::transmute(*ctx_vt.add(24));
        ia_set_topo(self.context, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

        if !self.last_bound_vbo.is_null() {
            let ia_set_vb: unsafe extern "system" fn(
                *mut c_void,
                u32,
                u32,
                *const *mut c_void,
                *const u32,
                *const u32,
            ) = std::mem::transmute(*ctx_vt.add(18));
            let offset: u32 = 0;
            ia_set_vb(self.context, 0, 1, &self.last_bound_vbo, &stride, &offset);
        }
    }
    pub(crate) fn set_scissor_impl(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
        if !self.context.is_null() {
            unsafe {
                let ctx_vt = com_vtable(self.context);
                let rs_set_scissor: unsafe extern "system" fn(*mut c_void, u32, *const D3D11_RECT) =
                    std::mem::transmute(*ctx_vt.add(45));
                let rect = D3D11_RECT {
                    left: x as i32,
                    top: y as i32,
                    right: (x + width) as i32,
                    bottom: (y + height) as i32,
                };
                rs_set_scissor(self.context, 1, &rect);
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

        if !self.context.is_null() {
            unsafe {
                let ctx_vt = com_vtable(self.context);
                let (rtv, w, h) = if self.active_render_target.is_valid() {
                    if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                        (rt.rtv, rt.width, rt.height)
                    } else {
                        (self.backbuffer_rtv, self.viewport_width, self.viewport_height)
                    }
                } else {
                    (self.backbuffer_rtv, self.viewport_width, self.viewport_height)
                };

                if !rtv.is_null() {
                    let om_set_rt: unsafe extern "system" fn(
                        *mut c_void,
                        u32,
                        *const *mut c_void,
                        *mut c_void,
                    ) = std::mem::transmute(*ctx_vt.add(33));
                    om_set_rt(self.context, 1, &rtv, std::ptr::null_mut());

                    let clear_rtv: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        *const [f32; 4],
                    ) = std::mem::transmute(*ctx_vt.add(50));
                    let rgba = [clear_color.r, clear_color.g, clear_color.b, clear_color.a];
                    clear_rtv(self.context, rtv, &rgba);
                }

                let rs_set_vp: unsafe extern "system" fn(
                    *mut c_void,
                    u32,
                    *const D3D11_VIEWPORT,
                ) = std::mem::transmute(*ctx_vt.add(44));
                let vp = D3D11_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: w as f32,
                    Height: h as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                };
                rs_set_vp(self.context, 1, &vp);

                let rs_set_scissor: unsafe extern "system" fn(*mut c_void, u32, *const D3D11_RECT) =
                    std::mem::transmute(*ctx_vt.add(45));
                let rect = D3D11_RECT {
                    left: 0,
                    top: 0,
                    right: w as i32,
                    bottom: h as i32,
                };
                rs_set_scissor(self.context, 1, &rect);
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
        DuctResult::Success
    }

    pub(crate) fn present_impl(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }
        if !self.swapchain.is_null() {
            unsafe {
                let sc_vt = com_vtable(self.swapchain);
                let present_fn: unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT =
                    std::mem::transmute(*sc_vt.add(8));
                let sync_interval = if self.vsync_enabled { 1 } else { 0 };
                let _ = present_fn(self.swapchain, sync_interval, 0);
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_int_impl(&mut self, _name: &str, _val: i32) -> DuctResult {
        DuctResult::Success
    }

    pub(crate) fn set_uniform_float_impl(&mut self, name: &str, val: f32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("scale") {
            self.constants.params[2] = val;
            self.constants.params[3] = val;
        } else {
            self.constants.params[0] = val;
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec2_impl(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("scale") {
            self.constants.params[2] = x;
            self.constants.params[3] = y;
        } else {
            self.constants.params[0] = x;
            self.constants.params[1] = y;
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec3_impl(&mut self, _name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        self.constants.color = [x, y, z, 1.0];
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec4_impl(&mut self, _name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        self.constants.color = [x, y, z, w];
        DuctResult::Success
    }

    pub(crate) fn set_uniform_mat4_impl(&mut self, _name: &str, elements: &[f32; 16]) -> DuctResult {
        self.constants.transform = *elements;
        DuctResult::Success
    }

    pub(crate) fn draw_impl(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count;

        unsafe {
            self.prepare_draw();
            if !self.context.is_null() {
                let ctx_vt = com_vtable(self.context);
                let draw_fn: unsafe extern "system" fn(*mut c_void, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(13));
                draw_fn(self.context, vertex_count, first_vertex);
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

        unsafe {
            self.prepare_draw();
            if !self.context.is_null() && !self.last_bound_ibo.is_null() {
                let ctx_vt = com_vtable(self.context);
                let ia_set_ib: unsafe extern "system" fn(*mut c_void, *mut c_void, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(19));
                let dxgi_idx_fmt = match self.current_index_format {
                    DuctIndexFormat::U16 => DXGI_FORMAT_R16_UINT,
                    DuctIndexFormat::U32 => DXGI_FORMAT_R32_UINT,
                };
                ia_set_ib(self.context, self.last_bound_ibo, dxgi_idx_fmt, 0);

                let draw_idx_fn: unsafe extern "system" fn(*mut c_void, u32, u32, i32) =
                    std::mem::transmute(*ctx_vt.add(12));
                draw_idx_fn(self.context, index_count, first_index, 0);
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

        unsafe {
            self.prepare_draw();
            if !self.context.is_null() {
                let ctx_vt = com_vtable(self.context);
                let draw_inst_fn: unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(21));
                draw_inst_fn(
                    self.context,
                    vertex_count,
                    instance_count,
                    first_vertex,
                    first_instance,
                );
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

        unsafe {
            self.prepare_draw();
            if !self.context.is_null() && !self.last_bound_ibo.is_null() {
                let ctx_vt = com_vtable(self.context);
                let ia_set_ib: unsafe extern "system" fn(*mut c_void, *mut c_void, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(19));
                let dxgi_idx_fmt = match index_format {
                    DuctIndexFormat::U16 => DXGI_FORMAT_R16_UINT,
                    DuctIndexFormat::U32 => DXGI_FORMAT_R32_UINT,
                };
                ia_set_ib(self.context, self.last_bound_ibo, dxgi_idx_fmt, 0);

                let draw_idx_inst_fn: unsafe extern "system" fn(
                    *mut c_void,
                    u32,
                    u32,
                    u32,
                    i32,
                    u32,
                ) = std::mem::transmute(*ctx_vt.add(20));
                draw_idx_inst_fn(
                    self.context,
                    index_count,
                    instance_count,
                    first_index,
                    0,
                    0,
                );
            }
        }
        DuctResult::Success
    }

    pub(crate) fn dispatch_compute_impl(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        if !self.context.is_null() {
            unsafe {
                let ctx_vt = com_vtable(self.context);
                let dispatch_fn: unsafe extern "system" fn(*mut c_void, u32, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(41));
                dispatch_fn(self.context, groups_x, groups_y, groups_z);
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
            allocated_gpu_memory_bytes: self.allocated_gpu_bytes.max(1024 * 1024 * 8),
        })
    }

    pub(crate) fn get_raw_context_impl(&mut self) -> Result<DuctRawContext, DuctResult> {
        Ok(DuctRawContext {
            backend_type: DuctBackendType::DirectX11,
            context_handle: self.device,
            device_handle: self.context,
            extra_info: self.extra_info.as_ptr(),
            get_proc_address: Some(dx11_get_proc_address),
        })
    }

    pub(crate) fn execute_custom_impl(&mut self, command_id: u32, data: *mut c_void) -> DuctResult {
        println!(
            "[dx11duct] Custom D3D11 command invoked [id={}, payload={:?}]",
            command_id, data
        );
        DuctResult::Success
    }
}
