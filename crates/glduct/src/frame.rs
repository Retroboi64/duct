use crate::gl::*;
use crate::types::*;
use duct_core::*;
use std::ffi::{c_void, CString};

impl GlPipeline {
    pub(crate) fn get_uniform_location(&mut self, name: &str) -> i32 {
        if self.active_program == 0 {
            return -1;
        }
        let prog = self.active_program;
        if let Some(locs) = self.uniform_locations.get(&prog) {
            if let Some(&loc) = locs.get(name) {
                return loc;
            }
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return -1,
        };
        let c_str = self
            .cached_uniform_cstrs
            .entry(name.to_string())
            .or_insert_with(|| CString::new(name).unwrap_or_default());
        let loc = unsafe { (gl.GetUniformLocation)(prog, c_str.as_ptr()) };
        self.uniform_locations
            .entry(prog)
            .or_default()
            .insert(name.to_string(), loc);
        loc
    }

    pub(crate) fn set_scissor_impl(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
        if let Some(gl) = &self.gl {
            unsafe {
                (gl.Enable)(GL_SCISSOR_TEST);
                (gl.Scissor)(x as i32, y as i32, width as i32, height as i32);
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

        if let Some(gl) = &self.gl {
            unsafe {
                (gl.ClearColor)(clear_color.r, clear_color.g, clear_color.b, clear_color.a);
                (gl.Clear)(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
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

        #[cfg(target_os = "windows")]
        if !self.hdc.is_null() {
            unsafe {
                win_wgl::SwapBuffers(self.hdc);
            }
        }

        #[cfg(target_os = "linux")]
        if !self.native_display.is_null() && !self.native_window.is_null() {
            unsafe {
                linux_glx::glXSwapBuffers(self.native_display, self.native_window as usize as u64);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let ctx = if !self.hglrc.is_null() {
                self.hglrc
            } else {
                unsafe { mac_cgl::CGLGetCurrentContext() }
            };
            if !ctx.is_null() {
                unsafe {
                    mac_cgl::CGLFlushDrawable(ctx);
                }
            }
        }

        DuctResult::Success
    }

    pub(crate) fn set_uniform_int_impl(&mut self, name: &str, val: i32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let loc = self.get_uniform_location(name);
        if loc >= 0 {
            if let Some(gl) = &self.gl {
                unsafe {
                    (gl.Uniform1i)(loc, val);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_float_impl(&mut self, name: &str, val: f32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let loc = self.get_uniform_location(name);
        if loc >= 0 {
            if let Some(gl) = &self.gl {
                unsafe {
                    (gl.Uniform1f)(loc, val);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec2_impl(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let loc = self.get_uniform_location(name);
        if loc >= 0 {
            if let Some(gl) = &self.gl {
                unsafe {
                    (gl.Uniform2f)(loc, x, y);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec3_impl(&mut self, name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let loc = self.get_uniform_location(name);
        if loc >= 0 {
            if let Some(gl) = &self.gl {
                unsafe {
                    (gl.Uniform3f)(loc, x, y, z);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_vec4_impl(&mut self, name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let loc = self.get_uniform_location(name);
        if loc >= 0 {
            if let Some(gl) = &self.gl {
                unsafe {
                    (gl.Uniform4f)(loc, x, y, z, w);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_uniform_mat4_impl(&mut self, name: &str, elements: &[f32; 16]) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let loc = self.get_uniform_location(name);
        if loc >= 0 {
            if let Some(gl) = &self.gl {
                unsafe {
                    (gl.UniformMatrix4fv)(loc, 1, GL_FALSE, elements.as_ptr());
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn draw_impl(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count;

        self.apply_vertex_layout_and_vbo(gl);
        unsafe {
            (gl.DrawArrays)(GL_TRIANGLES, first_vertex as i32, vertex_count as i32);
        }
        DuctResult::Success
    }

    pub(crate) fn draw_indexed_impl(&mut self, index_count: u32, first_index: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count;

        self.apply_vertex_layout_and_vbo(gl);
        let (gl_idx_type, elem_size) = match self.current_index_format {
            DuctIndexFormat::U16 => (GL_UNSIGNED_SHORT, 2),
            DuctIndexFormat::U32 => (GL_UNSIGNED_INT, 4),
        };
        unsafe {
            let offset = (first_index * elem_size) as usize as *const c_void;
            (gl.DrawElements)(GL_TRIANGLES, index_count as i32, gl_idx_type, offset);
        }
        DuctResult::Success
    }

    pub(crate) fn draw_instanced_impl(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        _first_instance: u32,
    ) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count * instance_count;

        self.apply_vertex_layout_and_vbo(gl);
        unsafe {
            (gl.DrawArraysInstanced)(
                GL_TRIANGLES,
                first_vertex as i32,
                vertex_count as i32,
                instance_count as i32,
            );
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
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count * instance_count;

        self.apply_vertex_layout_and_vbo(gl);
        let (gl_idx_type, elem_size) = match index_format {
            DuctIndexFormat::U16 => (GL_UNSIGNED_SHORT, 2),
            DuctIndexFormat::U32 => (GL_UNSIGNED_INT, 4),
        };
        unsafe {
            let offset = (first_index * elem_size) as usize as *const c_void;
            (gl.DrawElementsInstanced)(
                GL_TRIANGLES,
                index_count as i32,
                gl_idx_type,
                offset,
                instance_count as i32,
            );
        }
        DuctResult::Success
    }

    pub(crate) fn dispatch_compute_impl(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        unsafe {
            (gl.DispatchCompute)(groups_x.max(1), groups_y.max(1), groups_z.max(1));
            (gl.MemoryBarrier)(GL_ALL_BARRIER_BITS);
        }
        DuctResult::Success
    }

    pub(crate) fn get_frame_stats_impl(&mut self) -> Result<DuctFrameStats, DuctResult> {
        Ok(DuctFrameStats {
            fps: self.smoothed_fps,
            frame_time_ms: self.current_frame_time_ms,
            frame_count: self.frame_count,
            draw_calls: self.draw_calls_in_frame,
            triangles_rendered: (self.vertices_drawn_in_frame / 3) as u64,
            pipeline_switches: self.pipeline_switches_in_frame,
            allocated_gpu_memory_bytes: self.allocated_buffer_bytes,
        })
    }

    pub(crate) fn get_raw_context_impl(&mut self) -> Result<DuctRawContext, DuctResult> {
        let device_handle = if !self.hdc.is_null() {
            self.hdc
        } else {
            self.native_display
        };
        Ok(DuctRawContext {
            backend_type: DuctBackendType::OpenGL,
            context_handle: self.hglrc,
            device_handle,
            extra_info: self.extra_info.as_ptr(),
            get_proc_address: Some(gl_proc_loader),
        })
    }

    pub(crate) fn execute_custom_impl(&mut self, command_id: u32, data: *mut c_void) -> DuctResult {
        match command_id {
            1 => {
                println!(
                    "[glduct] Custom Command 1: GL Status [Shaders: {}, Buffers: {}, Total Frames: {}]",
                    self.shaders.len(),
                    self.buffers.len(),
                    self.frame_count
                );
                DuctResult::Success
            }
            2 => {
                let bitmask = data as usize as u32;
                if let Some(gl) = &self.gl {
                    unsafe { (gl.Clear)(bitmask) };
                }
                DuctResult::Success
            }
            _ => {
                println!("[glduct] Custom Command ID {} executed with payload {:?}.", command_id, data);
                DuctResult::Success
            }
        }
    }
}
