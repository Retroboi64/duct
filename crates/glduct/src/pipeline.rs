use crate::gl::*;
use crate::types::*;
use duct_core::*;
use std::ffi::{c_char, c_void};

impl GlPipeline {
    pub(crate) fn apply_vertex_layout_and_vbo(&self, gl: &Gl) {
        unsafe {
            if self.default_vao != 0 {
                (gl.BindVertexArray)(self.default_vao);
            }
            if self.last_bound_vbo != 0 {
                (gl.BindBuffer)(GL_ARRAY_BUFFER, self.last_bound_vbo);
                if self.current_vertex_layout.attribute_count > 0 {
                    let stride = self.current_vertex_layout.stride as i32;
                    let count = (self.current_vertex_layout.attribute_count as usize)
                        .min(self.current_vertex_layout.attributes.len());
                    for attr in &self.current_vertex_layout.attributes[..count] {
                        let (size, gl_type, normalized) = match attr.format {
                            DuctVertexFormat::Float => (1, GL_FLOAT, GL_FALSE),
                            DuctVertexFormat::Float2 => (2, GL_FLOAT, GL_FALSE),
                            DuctVertexFormat::Float3 => (3, GL_FLOAT, GL_FALSE),
                            DuctVertexFormat::Float4 => (4, GL_FLOAT, GL_FALSE),
                            DuctVertexFormat::UByte4Normalized => (4, GL_UNSIGNED_BYTE, GL_TRUE),
                        };
                        (gl.VertexAttribPointer)(
                            attr.location,
                            size,
                            gl_type,
                            normalized,
                            stride,
                            attr.offset as usize as *const c_void,
                        );
                        (gl.EnableVertexAttribArray)(attr.location);
                    }
                } else {
                    // Fallback to location 0, 3 floats
                    (gl.VertexAttribPointer)(
                        0,
                        3,
                        GL_FLOAT,
                        GL_FALSE,
                        3 * std::mem::size_of::<f32>() as i32,
                        std::ptr::null(),
                    );
                    (gl.EnableVertexAttribArray)(0);
                }
            }
            if self.last_bound_ebo != 0 {
                (gl.BindBuffer)(GL_ELEMENT_ARRAY_BUFFER, self.last_bound_ebo);
            }
        }
    }
    pub(crate) fn create_shader_impl(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        let gl = match &self.gl {
            Some(g) => g,
            None => return Err(DuctResult::ErrorInvalidState),
        };

        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return Err(DuctResult::ErrorInvalidParameter);
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut source_bytes = slice.to_vec();
        if !source_bytes.ends_with(&[0]) {
            source_bytes.push(0);
        }

        let gl_stage = match desc.stage {
            DuctShaderStage::Vertex => GL_VERTEX_SHADER,
            DuctShaderStage::Fragment => GL_FRAGMENT_SHADER,
            DuctShaderStage::Compute => GL_COMPUTE_SHADER,
        };

        let gl_id = unsafe { (gl.CreateShader)(gl_stage) };
        if gl_id == 0 {
            return Err(DuctResult::ErrorInitializationFailed);
        }

        let ptr = source_bytes.as_ptr() as *const c_char;
        unsafe {
            (gl.ShaderSource)(gl_id, 1, &ptr, std::ptr::null());
            (gl.CompileShader)(gl_id);

            let mut success = 0i32;
            (gl.GetShaderiv)(gl_id, GL_COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut log_len = 0i32;
                (gl.GetShaderiv)(gl_id, GL_INFO_LOG_LENGTH, &mut log_len);
                let mut buffer = vec![0u8; log_len as usize];
                (gl.GetShaderInfoLog)(
                    gl_id,
                    log_len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut c_char,
                );
                eprintln!(
                    "[glduct][ERROR] Shader compile failure ({:?}):\n{}",
                    desc.stage,
                    String::from_utf8_lossy(&buffer)
                );
                (gl.DeleteShader)(gl_id);
                return Err(DuctResult::ErrorInvalidParameter);
            }
        }

        let handle = DuctShaderHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.shaders.insert(handle, GlShaderRecord {
            gl_id,
            _stage: desc.stage,
        });

        Ok(handle)
    }

    pub(crate) fn reload_shader_impl(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        let existing = match self.shaders.get(&handle) {
            Some(s) => s,
            None => return DuctResult::ErrorResourceNotFound,
        };

        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return DuctResult::ErrorInvalidParameter;
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut source_bytes = slice.to_vec();
        if !source_bytes.ends_with(&[0]) {
            source_bytes.push(0);
        }

        let gl_stage = match desc.stage {
            DuctShaderStage::Vertex => GL_VERTEX_SHADER,
            DuctShaderStage::Fragment => GL_FRAGMENT_SHADER,
            DuctShaderStage::Compute => GL_COMPUTE_SHADER,
        };

        let new_gl_id = unsafe { (gl.CreateShader)(gl_stage) };
        if new_gl_id == 0 {
            return DuctResult::ErrorInitializationFailed;
        }

        let ptr = source_bytes.as_ptr() as *const c_char;
        unsafe {
            (gl.ShaderSource)(new_gl_id, 1, &ptr, std::ptr::null());
            (gl.CompileShader)(new_gl_id);

            let mut success = 0i32;
            (gl.GetShaderiv)(new_gl_id, GL_COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut log_len = 0i32;
                (gl.GetShaderiv)(new_gl_id, GL_INFO_LOG_LENGTH, &mut log_len);
                let mut buffer = vec![0u8; log_len as usize];
                (gl.GetShaderInfoLog)(
                    new_gl_id,
                    log_len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut c_char,
                );
                eprintln!(
                    "[glduct][WARN] Shader reload failed for {:?}:\n{}",
                    handle,
                    String::from_utf8_lossy(&buffer)
                );
                (gl.DeleteShader)(new_gl_id);
                return DuctResult::ErrorInvalidParameter;
            }

            // Success: delete old shader
            (gl.DeleteShader)(existing.gl_id);
        }

        self.shaders.insert(handle, GlShaderRecord {
            gl_id: new_gl_id,
            _stage: desc.stage,
        });

        // Invalidate cached programs that use this shader so they are relinked
        let uniform_locs = &mut self.uniform_locations;
        self.programs.retain(|&(vs, fs), &mut prog| {
            if vs == handle || fs == handle {
                uniform_locs.remove(&prog);
                unsafe { (gl.DeleteProgram)(prog); }
                false
            } else {
                true
            }
        });

        println!("[glduct] Shader {:?} reloaded successfully!", handle);
        DuctResult::Success
    }

    pub(crate) fn destroy_shader_impl(&mut self, handle: DuctShaderHandle) -> DuctResult {
        if let Some(record) = self.shaders.remove(&handle) {
            if let Some(gl) = &self.gl {
                unsafe {
                    (gl.DeleteShader)(record.gl_id);
                }
            }
            let gl_opt = self.gl.as_ref();
            let uniform_locs = &mut self.uniform_locations;
            self.programs.retain(|&(vs, fs), &mut prog| {
                if vs == handle || fs == handle {
                    uniform_locs.remove(&prog);
                    if let Some(gl) = gl_opt {
                        unsafe { (gl.DeleteProgram)(prog); }
                    }
                    false
                } else {
                    true
                }
            });
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn set_pipeline_state_impl(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        let vs_rec = match self.shaders.get(&desc.vertex_shader) {
            Some(s) => s,
            None => return DuctResult::ErrorResourceNotFound,
        };

        // Support binding a standalone Compute Shader via vertex_shader slot when fragment_shader is INVALID
        if vs_rec._stage == DuctShaderStage::Compute && !desc.fragment_shader.is_valid() {
            let cs_id = vs_rec.gl_id;
            let program = if let Some(&p) = self.compute_programs.get(&desc.vertex_shader) {
                p
            } else {
                let prog = unsafe { (gl.CreateProgram)() };
                unsafe {
                    (gl.AttachShader)(prog, cs_id);
                    (gl.LinkProgram)(prog);
                    let mut ok = 0i32;
                    (gl.GetProgramiv)(prog, GL_LINK_STATUS, &mut ok);
                    if ok == 0 {
                        (gl.DeleteProgram)(prog);
                        return DuctResult::ErrorInitializationFailed;
                    }
                }
                self.compute_programs.insert(desc.vertex_shader, prog);
                prog
            };
            unsafe {
                (gl.UseProgram)(program);
            }
            self.active_program = program;
            self.pipeline_switches_in_frame += 1;
            return DuctResult::Success;
        }

        let vs = vs_rec.gl_id;
        let fs = match self.shaders.get(&desc.fragment_shader) {
            Some(s) => s.gl_id,
            None => return DuctResult::ErrorResourceNotFound,
        };

        self.current_vertex_layout = desc.vertex_layout;
        self.current_depth_compare = desc.depth_compare;
        self.pipeline_switches_in_frame += 1;

        let pair = (desc.vertex_shader, desc.fragment_shader);
        let program = if let Some(&p) = self.programs.get(&pair) {
            p
        } else {
            let prog = unsafe { (gl.CreateProgram)() };
            unsafe {
                (gl.AttachShader)(prog, vs);
                (gl.AttachShader)(prog, fs);
                (gl.LinkProgram)(prog);

                let mut ok = 0i32;
                (gl.GetProgramiv)(prog, GL_LINK_STATUS, &mut ok);
                if ok == 0 {
                    let mut log_len = 0i32;
                    (gl.GetProgramiv)(prog, GL_INFO_LOG_LENGTH, &mut log_len);
                    let mut buffer = vec![0u8; log_len as usize];
                    (gl.GetProgramInfoLog)(
                        prog,
                        log_len,
                        std::ptr::null_mut(),
                        buffer.as_mut_ptr() as *mut c_char,
                    );
                    eprintln!(
                        "[glduct][ERROR] Shader program link failure:\n{}",
                        String::from_utf8_lossy(&buffer)
                    );
                    (gl.DeleteProgram)(prog);
                    return DuctResult::ErrorInitializationFailed;
                }
            }
            self.programs.insert(pair, prog);
            prog
        };

        unsafe {
            (gl.UseProgram)(program);
            self.active_program = program;

            // Face Culling
            match desc.cull_mode {
                DuctCullMode::None => (gl.Disable)(GL_CULL_FACE),
                DuctCullMode::Front => {
                    (gl.Enable)(GL_CULL_FACE);
                    (gl.CullFace)(GL_FRONT);
                }
                DuctCullMode::Back => {
                    (gl.Enable)(GL_CULL_FACE);
                    (gl.CullFace)(GL_BACK);
                }
            }

            // Blending
            match desc.blend_mode {
                DuctBlendMode::Opaque => (gl.Disable)(GL_BLEND),
                DuctBlendMode::AlphaBlend => {
                    (gl.Enable)(GL_BLEND);
                    (gl.BlendFunc)(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
                }
                DuctBlendMode::Additive => {
                    (gl.Enable)(GL_BLEND);
                    (gl.BlendFunc)(GL_SRC_ALPHA, GL_ONE);
                }
            }

            // Depth
            if desc.depth_test {
                (gl.Enable)(GL_DEPTH_TEST);
            } else {
                (gl.Disable)(GL_DEPTH_TEST);
            }
            (gl.DepthMask)(if desc.depth_write { GL_TRUE } else { GL_FALSE });
            (gl.DepthFunc)(map_compare_op(desc.depth_compare));
        }

        DuctResult::Success
    }

    pub(crate) fn set_wireframe_impl(&mut self, enabled: bool) -> DuctResult {
        if let Some(gl) = &self.gl {
            unsafe {
                (gl.PolygonMode)(GL_FRONT_AND_BACK, if enabled { GL_LINE } else { GL_FILL });
            }
        }
        DuctResult::Success
    }

}
