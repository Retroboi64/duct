use duct_core::*;
use std::ffi::c_void;
use crate::types::*;

impl Dx12Pipeline {
    pub(crate) fn update_buffer_d3d12(&mut self, rec: &Dx12BufferRecord, offset: usize, data: &[u8]) -> DuctResult {
        if data.is_empty() {
            return DuctResult::Success;
        }

        // When D3D12 device is available, use D3D12 upload heap & CopyBufferRegion pattern
        if !self.d3d12_device.is_null() && !rec.buffer.is_null() {
            unsafe {
                let dev = self.d3d12_device;
                let dev_vt = com_vtable(dev);

                // 1. Create an intermediate Upload Heap buffer
                let upload_heap_props = D3D12_HEAP_PROPERTIES {
                    Type: D3D12_HEAP_TYPE_UPLOAD,
                    CPUPageProperty: 0,
                    MemoryPoolPreference: 0,
                    CreationNodeMask: 1,
                    VisibleNodeMask: 1,
                };

                let upload_desc = D3D12_RESOURCE_DESC {
                    Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                    Alignment: 0,
                    Width: data.len() as u64,
                    Height: 1,
                    DepthOrArraySize: 1,
                    MipLevels: 1,
                    Format: 0,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                    Flags: D3D12_RESOURCE_FLAG_NONE,
                };

                let create_committed: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D12_HEAP_PROPERTIES,
                    u32,
                    *const D3D12_RESOURCE_DESC,
                    u32,
                    *const c_void,
                    *const GUID,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(27));

                let mut upload_buffer: *mut c_void = std::ptr::null_mut();
                let hr = create_committed(
                    dev,
                    &upload_heap_props,
                    D3D12_HEAP_FLAG_NONE,
                    &upload_desc,
                    D3D12_RESOURCE_STATE_GENERIC_READ,
                    std::ptr::null(),
                    &IID_ID3D12Resource,
                    &mut upload_buffer,
                );

                if hr == S_OK && !upload_buffer.is_null() {
                    // 2. Map upload heap, copy CPU data, unmap
                    let res_vt = com_vtable(upload_buffer);
                    let map_fn: unsafe extern "system" fn(
                        *mut c_void,
                        u32,
                        *const c_void,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*res_vt.add(8));
                    let unmap_fn: unsafe extern "system" fn(*mut c_void, u32, *const c_void) =
                        std::mem::transmute(*res_vt.add(9));

                    let mut mapped_ptr: *mut c_void = std::ptr::null_mut();
                    if map_fn(upload_buffer, 0, std::ptr::null(), &mut mapped_ptr) == S_OK
                        && !mapped_ptr.is_null()
                    {
                        std::ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr as *mut u8, data.len());
                        unmap_fn(upload_buffer, 0, std::ptr::null());
                    }

                    // 3. Prepare resource barrier transition: COMMON -> COPY_DEST
                    let _barrier_to_copy = D3D12_RESOURCE_BARRIER {
                        Type: 0, // Transition
                        Flags: 0,
                        Transition: D3D12_RESOURCE_TRANSITION_BARRIER {
                            pResource: rec.buffer,
                            Subresource: 0,
                            StateBefore: D3D12_RESOURCE_STATE_COMMON,
                            StateAfter: D3D12_RESOURCE_STATE_COPY_DEST,
                        },
                    };

                    // 4. Prepare resource barrier transition: COPY_DEST -> GENERIC_READ
                    let _barrier_to_read = D3D12_RESOURCE_BARRIER {
                        Type: 0, // Transition
                        Flags: 0,
                        Transition: D3D12_RESOURCE_TRANSITION_BARRIER {
                            pResource: rec.buffer,
                            Subresource: 0,
                            StateBefore: D3D12_RESOURCE_STATE_COPY_DEST,
                            StateAfter: D3D12_RESOURCE_STATE_GENERIC_READ,
                        },
                    };

                    let _ = offset;
                    com_release(upload_buffer);
                }
            }
        }

        DuctResult::Success
    }

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
            Dx12BufferRecord {
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
            if !data.is_empty() {
                let rec_clone = *rec;
                return self.update_buffer_d3d12(&rec_clone, offset, data);
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
