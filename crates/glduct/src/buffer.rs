use crate::gl::*;
use crate::types::*;
use duct_core::*;
use std::ffi::c_void;

impl GlPipeline {
    pub(crate) fn create_buffer_impl(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
        let gl = match &self.gl {
            Some(g) => g,
            None => return Err(DuctResult::ErrorInvalidState),
        };

        let gl_target = match desc.buffer_type {
            DuctBufferType::Vertex => GL_ARRAY_BUFFER,
            DuctBufferType::Index => GL_ELEMENT_ARRAY_BUFFER,
            DuctBufferType::Uniform => GL_UNIFORM_BUFFER,
            DuctBufferType::Storage => GL_SHADER_STORAGE_BUFFER,
        };

        let usage = if desc.is_dynamic { GL_DYNAMIC_DRAW } else { GL_STATIC_DRAW };

        let mut gl_id = 0u32;
        unsafe {
            (gl.GenBuffers)(1, &mut gl_id);
            (gl.BindBuffer)(gl_target, gl_id);
            let ptr = if !desc.initial_data.is_null() {
                desc.initial_data as *const c_void
            } else {
                std::ptr::null()
            };
            (gl.BufferData)(gl_target, desc.size_bytes as isize, ptr, usage);
        }

        if desc.buffer_type == DuctBufferType::Vertex {
            self.last_bound_vbo = gl_id;
        } else if desc.buffer_type == DuctBufferType::Index {
            self.last_bound_ebo = gl_id;
        }

        self.allocated_buffer_bytes += desc.size_bytes;

        let handle = DuctBufferHandle(self.next_handle_id);
        self.next_handle_id += 1;

        self.buffers.insert(handle, GlBufferRecord {
            gl_id,
            gl_target,
            _size_bytes: desc.size_bytes,
        });

        Ok(handle)
    }

    pub(crate) fn update_buffer_impl(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        if let Some(buf) = self.buffers.get(&handle) {
            unsafe {
                (gl.BindBuffer)(buf.gl_target, buf.gl_id);
                (gl.BufferSubData)(
                    buf.gl_target,
                    offset as isize,
                    data.len() as isize,
                    data.as_ptr() as *const c_void,
                );
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn destroy_buffer_impl(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if let Some(buf) = self.buffers.remove(&handle) {
            self.allocated_buffer_bytes = self.allocated_buffer_bytes.saturating_sub(buf._size_bytes);
            if let Some(gl) = &self.gl {
                let mut id = buf.gl_id;
                unsafe {
                    (gl.DeleteBuffers)(1, &mut id);
                }
            }
            if self.last_bound_vbo == buf.gl_id {
                self.last_bound_vbo = 0;
            }
            if self.last_bound_ebo == buf.gl_id {
                self.last_bound_ebo = 0;
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_vertex_buffer_impl(&mut self, handle: DuctBufferHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(buf) = self.buffers.get(&handle) {
            self.last_bound_vbo = buf.gl_id;
            unsafe {
                (gl.BindBuffer)(GL_ARRAY_BUFFER, buf.gl_id);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_index_buffer_impl(&mut self, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(buf) = self.buffers.get(&handle) {
            self.last_bound_ebo = buf.gl_id;
            self.current_index_format = format;
            unsafe {
                if self.default_vao != 0 {
                    (gl.BindVertexArray)(self.default_vao);
                }
                (gl.BindBuffer)(GL_ELEMENT_ARRAY_BUFFER, buf.gl_id);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_uniform_buffer_impl(&mut self, slot: u32, handle: DuctBufferHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(buf) = self.buffers.get(&handle) {
            unsafe {
                (gl.BindBufferBase)(buf.gl_target, slot, buf.gl_id);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

}
