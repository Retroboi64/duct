use crate::types::*;
use crate::types::VkPipeline;
use crate::vk::*;
use duct_core::*;
use std::ffi::c_void;

impl VkPipeline {
    pub(crate) fn create_buffer_impl(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
        let mut vk_buf = VK_NULL_HANDLE;
        let mut vk_mem = VK_NULL_HANDLE;

        if let Some(dfns) = &self.device_fns {
            if !self.device.is_null() && desc.size_bytes > 0 {
                let usage = match desc.buffer_type {
                    DuctBufferType::Vertex => VK_BUFFER_USAGE_VERTEX_BUFFER_BIT,
                    DuctBufferType::Index => VK_BUFFER_USAGE_INDEX_BUFFER_BIT,
                    DuctBufferType::Uniform => VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT,
                    DuctBufferType::Storage => VK_BUFFER_USAGE_STORAGE_BUFFER_BIT,
                } | VK_BUFFER_USAGE_TRANSFER_DST_BIT
                    | VK_BUFFER_USAGE_TRANSFER_SRC_BIT;

                let buf_info = VkBufferCreateInfo {
                    sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                    size: desc.size_bytes as VkDeviceSize,
                    usage,
                    sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                    queueFamilyIndexCount: 0,
                    pQueueFamilyIndices: std::ptr::null(),
                };

                if unsafe {
                    (dfns.CreateBuffer)(self.device, &buf_info, std::ptr::null(), &mut vk_buf)
                } == VK_SUCCESS
                {
                    let mut mem_reqs = VkMemoryRequirements::default();
                    unsafe {
                        (dfns.GetBufferMemoryRequirements)(self.device, vk_buf, &mut mem_reqs);
                    }

                    if let Some(mem_type_idx) = self.find_memory_type(
                        mem_reqs.memoryTypeBits,
                        VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                    ) {
                        let alloc_info = VkMemoryAllocateInfo {
                            sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                            pNext: std::ptr::null(),
                            allocationSize: mem_reqs.size,
                            memoryTypeIndex: mem_type_idx,
                        };
                        if unsafe {
                            (dfns.AllocateMemory)(
                                self.device,
                                &alloc_info,
                                std::ptr::null(),
                                &mut vk_mem,
                            )
                        } == VK_SUCCESS
                        {
                            unsafe {
                                let _ = (dfns.BindBufferMemory)(self.device, vk_buf, vk_mem, 0);
                                if !desc.initial_data.is_null() {
                                    let mut mapped: *mut c_void = std::ptr::null_mut();
                                    if (dfns.MapMemory)(
                                        self.device,
                                        vk_mem,
                                        0,
                                        desc.size_bytes as VkDeviceSize,
                                        0,
                                        &mut mapped,
                                    ) == VK_SUCCESS
                                        && !mapped.is_null()
                                    {
                                        std::ptr::copy_nonoverlapping(
                                            desc.initial_data,
                                            mapped as *mut u8,
                                            desc.size_bytes,
                                        );
                                        (dfns.UnmapMemory)(self.device, vk_mem);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if desc.buffer_type == DuctBufferType::Vertex && vk_buf != VK_NULL_HANDLE {
            self.last_bound_vbo = vk_buf;
        } else if desc.buffer_type == DuctBufferType::Index && vk_buf != VK_NULL_HANDLE {
            self.last_bound_ibo = vk_buf;
        }

        self.allocated_gpu_bytes += desc.size_bytes;

        let handle = DuctBufferHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.buffers.insert(
            handle,
            VkBufferRecord {
                buffer: vk_buf,
                memory: vk_mem,
                size_bytes: desc.size_bytes,
                _buffer_type: desc.buffer_type,
            },
        );
        Ok(handle)
    }

    pub(crate) fn update_buffer_impl(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            if offset + data.len() > rec.size_bytes {
                return DuctResult::ErrorInvalidParameter;
            }
            if rec.memory != VK_NULL_HANDLE && !data.is_empty() {
                if let Some(dfns) = &self.device_fns {
                    unsafe {
                        let mut mapped: *mut c_void = std::ptr::null_mut();
                        if (dfns.MapMemory)(
                            self.device,
                            rec.memory,
                            0,
                            rec.size_bytes as VkDeviceSize,
                            0,
                            &mut mapped,
                        ) == VK_SUCCESS
                            && !mapped.is_null()
                        {
                            std::ptr::copy_nonoverlapping(
                                data.as_ptr(),
                                (mapped as *mut u8).add(offset),
                                data.len(),
                            );
                            (dfns.UnmapMemory)(self.device, rec.memory);
                        }
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
                self.last_bound_vbo = VK_NULL_HANDLE;
            }
            if self.last_bound_ibo == rec.buffer {
                self.last_bound_ibo = VK_NULL_HANDLE;
            }
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        let _ = (dfns.DeviceWaitIdle)(self.device);
                        if rec.buffer != VK_NULL_HANDLE {
                            (dfns.DestroyBuffer)(self.device, rec.buffer, std::ptr::null());
                        }
                        if rec.memory != VK_NULL_HANDLE {
                            (dfns.FreeMemory)(self.device, rec.memory, std::ptr::null());
                        }
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_vertex_buffer_impl(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            self.last_bound_vbo = rec.buffer;
            if self.cmd_recording && rec.buffer != VK_NULL_HANDLE {
                if let Some(dfns) = &self.device_fns {
                    let offset: VkDeviceSize = 0;
                    unsafe {
                        (dfns.CmdBindVertexBuffers)(
                            self.command_buffer,
                            0,
                            1,
                            &rec.buffer,
                            &offset,
                        );
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_index_buffer_impl(&mut self, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            self.last_bound_ibo = rec.buffer;
            self.current_index_format = format;
            if self.cmd_recording && rec.buffer != VK_NULL_HANDLE {
                if let Some(dfns) = &self.device_fns {
                    let vk_idx = match format {
                        DuctIndexFormat::U16 => VK_INDEX_TYPE_UINT16,
                        DuctIndexFormat::U32 => VK_INDEX_TYPE_UINT32,
                    };
                    unsafe {
                        (dfns.CmdBindIndexBuffer)(self.command_buffer, rec.buffer, 0, vk_idx);
                    }
                }
            }
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
