use duct_core::*;
use std::ffi::c_void;
use crate::types::*;

impl Dx11Pipeline {
    pub(crate) fn create_buffer_impl(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
        let mut buf_ptr: *mut c_void = std::ptr::null_mut();

        if !self.device.is_null() && desc.size_bytes > 0 {
            let bind_flags = match desc.buffer_type {
                DuctBufferType::Vertex => D3D11_BIND_VERTEX_BUFFER,
                DuctBufferType::Index => D3D11_BIND_INDEX_BUFFER,
                DuctBufferType::Uniform => D3D11_BIND_CONSTANT_BUFFER,
                DuctBufferType::Storage => D3D11_BIND_SHADER_RESOURCE,
            };
            let byte_width = if desc.buffer_type == DuctBufferType::Uniform {
                ((desc.size_bytes + 15) & !15).max(16) as u32
            } else {
                desc.size_bytes as u32
            };

            let bdesc = D3D11_BUFFER_DESC {
                ByteWidth: byte_width,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: bind_flags,
                CPUAccessFlags: 0,
                MiscFlags: 0,
                StructureByteStride: 0,
            };

            unsafe {
                let dev_vt = com_vtable(self.device);
                let create_buf: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_BUFFER_DESC,
                    *const D3D11_SUBRESOURCE_DATA,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(3));

                let mut padded_vec = Vec::new();
                let init_data_ptr = if !desc.initial_data.is_null() {
                    if byte_width as usize > desc.size_bytes {
                        padded_vec.resize(byte_width as usize, 0u8);
                        std::ptr::copy_nonoverlapping(
                            desc.initial_data,
                            padded_vec.as_mut_ptr(),
                            desc.size_bytes,
                        );
                        padded_vec.as_ptr() as *const c_void
                    } else {
                        desc.initial_data as *const c_void
                    }
                } else {
                    std::ptr::null()
                };

                let sub_data = D3D11_SUBRESOURCE_DATA {
                    pSysMem: init_data_ptr,
                    SysMemPitch: 0,
                    SysMemSlicePitch: 0,
                };
                let _ = create_buf(
                    self.device,
                    &bdesc,
                    if init_data_ptr.is_null() {
                        std::ptr::null()
                    } else {
                        &sub_data
                    },
                    &mut buf_ptr,
                );
            }
        }

        if desc.buffer_type == DuctBufferType::Vertex && !buf_ptr.is_null() {
            self.last_bound_vbo = buf_ptr;
        } else if desc.buffer_type == DuctBufferType::Index && !buf_ptr.is_null() {
            self.last_bound_ibo = buf_ptr;
        }

        self.allocated_gpu_bytes += desc.size_bytes;
        let handle = DuctBufferHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.buffers.insert(
            handle,
            Dx11BufferRecord {
                buffer: buf_ptr,
                size_bytes: desc.size_bytes,
                buffer_type: desc.buffer_type,
            },
        );
        Ok(handle)
    }

    pub(crate) fn update_buffer_impl(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            if offset + data.len() > rec.size_bytes {
                return DuctResult::ErrorInvalidParameter;
            }
            if !rec.buffer.is_null() && !self.context.is_null() && !data.is_empty() {
                unsafe {
                    let ctx_vt = com_vtable(self.context);
                    let update_subres: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        u32,
                        *const D3D11_BOX,
                        *const c_void,
                        u32,
                        u32,
                    ) = std::mem::transmute(*ctx_vt.add(48));

                    if rec.buffer_type == DuctBufferType::Uniform {
                        let byte_width = ((rec.size_bytes + 15) & !15).max(16);
                        let mut padded = vec![0u8; byte_width];
                        let copy_len = data.len().min(byte_width.saturating_sub(offset));
                        padded[offset..offset + copy_len].copy_from_slice(&data[..copy_len]);
                        update_subres(
                            self.context,
                            rec.buffer,
                            0,
                            std::ptr::null(),
                            padded.as_ptr() as *const c_void,
                            0,
                            0,
                        );
                    } else {
                        let dst_box = D3D11_BOX {
                            left: offset as u32,
                            top: 0,
                            front: 0,
                            right: (offset + data.len()) as u32,
                            bottom: 1,
                            back: 1,
                        };
                        update_subres(
                            self.context,
                            rec.buffer,
                            0,
                            &dst_box,
                            data.as_ptr() as *const c_void,
                            0,
                            0,
                        );
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn destroy_buffer_impl(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if let Some(rec) = self.buffers.remove(&handle) {
            self.allocated_gpu_bytes = self.allocated_gpu_bytes.saturating_sub(rec.size_bytes);
            if self.last_bound_vbo == rec.buffer {
                self.last_bound_vbo = std::ptr::null_mut();
            }
            if self.last_bound_ibo == rec.buffer {
                self.last_bound_ibo = std::ptr::null_mut();
            }
            unsafe {
                com_release(rec.buffer);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_vertex_buffer_impl(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            self.last_bound_vbo = rec.buffer;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_index_buffer_impl(&mut self, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            self.last_bound_ibo = rec.buffer;
            self.current_index_format = format;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_uniform_buffer_impl(&mut self, _slot: u32, handle: DuctBufferHandle) -> DuctResult {
        if self.buffers.contains_key(&handle) {
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

}
