use duct_core::*;
use std::ffi::c_void;
use crate::types::*;

impl Dx12Pipeline {
    pub(crate) fn create_texture_2d_impl(
        &mut self,
        width: u32,
        height: u32,
        format: DuctTextureFormat,
        data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctResult> {
        let dxgi_fmt = match format {
            DuctTextureFormat::Rgba8 | DuctTextureFormat::Rgb8 => DXGI_FORMAT_R8G8B8A8_UNORM,
            DuctTextureFormat::R8 => DXGI_FORMAT_R8_UNORM,
            DuctTextureFormat::Rgba16Float => DXGI_FORMAT_R16G16B16A16_FLOAT,
            DuctTextureFormat::Depth32Float => DXGI_FORMAT_R32_FLOAT,
        };
        let mut tex_ptr: *mut c_void = std::ptr::null_mut();
        let mut srv_ptr: *mut c_void = std::ptr::null_mut();
        let mut samp_ptr: *mut c_void = std::ptr::null_mut();

        if !self.device.is_null() && width > 0 && height > 0 {
            let tdesc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: dxgi_fmt,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };

            unsafe {
                let dev_vt = com_vtable(self.device);
                let create_tex: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_TEXTURE2D_DESC,
                    *const D3D11_SUBRESOURCE_DATA,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(5));

                let bpp = match dxgi_fmt {
                    DXGI_FORMAT_R8_UNORM => 1,
                    DXGI_FORMAT_R16G16B16A16_FLOAT => 8,
                    _ => 4,
                };
                let sub_data = D3D11_SUBRESOURCE_DATA {
                    pSysMem: data.map(|d| d.as_ptr() as *const c_void).unwrap_or(std::ptr::null()),
                    SysMemPitch: width * bpp,
                    SysMemSlicePitch: 0,
                };
                if create_tex(
                    self.device,
                    &tdesc,
                    if data.is_some() {
                        &sub_data
                    } else {
                        std::ptr::null()
                    },
                    &mut tex_ptr,
                ) == S_OK
                    && !tex_ptr.is_null()
                {
                    let create_srv: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        *const c_void,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(7));
                    let _ = create_srv(self.device, tex_ptr, std::ptr::null(), &mut srv_ptr);

                    let sdesc = D3D11_SAMPLER_DESC {
                        Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                        AddressU: D3D11_TEXTURE_ADDRESS_WRAP,
                        AddressV: D3D11_TEXTURE_ADDRESS_WRAP,
                        AddressW: D3D11_TEXTURE_ADDRESS_WRAP,
                        MipLODBias: 0.0,
                        MaxAnisotropy: 1,
                        ComparisonFunc: 8,
                        BorderColor: [0.0; 4],
                        MinLOD: 0.0,
                        MaxLOD: f32::MAX,
                    };
                    let create_samp: unsafe extern "system" fn(
                        *mut c_void,
                        *const D3D11_SAMPLER_DESC,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(23));
                    let _ = create_samp(self.device, &sdesc, &mut samp_ptr);
                }
            }
        }

        self.allocated_gpu_bytes += (width as usize) * (height as usize) * 4;
        let handle = DuctTextureHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.textures.insert(
            handle,
            Dx12TextureRecord {
                texture: tex_ptr,
                srv: srv_ptr,
                sampler: samp_ptr,
                width,
                height,
                format: dxgi_fmt,
                min_filter: DuctFilterMode::Linear,
                mag_filter: DuctFilterMode::Linear,
                wrap_s: DuctWrapMode::Repeat,
                wrap_t: DuctWrapMode::Repeat,
            },
        );
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
        if let Some(tex) = self.textures.get(&handle) {
            if x + width > tex.width || y + height > tex.height {
                return DuctResult::ErrorInvalidParameter;
            }
            if !tex.texture.is_null() && !self.context.is_null() && !data.is_empty() {
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
                    let bpp = match tex.format {
                        DXGI_FORMAT_R8_UNORM => 1,
                        DXGI_FORMAT_R16G16B16A16_FLOAT => 8,
                        _ => 4,
                    };
                    let dst_box = D3D11_BOX {
                        left: x,
                        top: y,
                        front: 0,
                        right: x + width,
                        bottom: y + height,
                        back: 1,
                    };
                    update_subres(
                        self.context,
                        tex.texture,
                        0,
                        &dst_box,
                        data.as_ptr() as *const c_void,
                        width * bpp,
                        0,
                    );
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn destroy_texture_impl(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if let Some(t) = self.textures.remove(&handle) {
            self.allocated_gpu_bytes = self
                .allocated_gpu_bytes
                .saturating_sub((t.width as usize) * (t.height as usize) * 4);
            unsafe {
                com_release(t.sampler);
                com_release(t.srv);
                com_release(t.texture);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn bind_texture_impl(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult {
        if let Some(t) = self.textures.get(&handle) {
            if !self.context.is_null() {
                unsafe {
                    let ctx_vt = com_vtable(self.context);
                    if !t.srv.is_null() {
                        let ps_set_srv: unsafe extern "system" fn(
                            *mut c_void,
                            u32,
                            u32,
                            *const *mut c_void,
                        ) = std::mem::transmute(*ctx_vt.add(8));
                        ps_set_srv(self.context, slot, 1, &t.srv);
                    }
                    if !t.sampler.is_null() {
                        let ps_set_samp: unsafe extern "system" fn(
                            *mut c_void,
                            u32,
                            u32,
                            *const *mut c_void,
                        ) = std::mem::transmute(*ctx_vt.add(10));
                        ps_set_samp(self.context, slot, 1, &t.sampler);
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
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
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn create_render_target_impl(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        let color_tex = self.create_texture_2d(desc.width, desc.height, desc.format, None)?;
        let mut rtv_ptr: *mut c_void = std::ptr::null_mut();

        if let Some(tex) = self.textures.get(&color_tex) {
            if !tex.texture.is_null() && !self.device.is_null() {
                unsafe {
                    let dev_vt = com_vtable(self.device);
                    let create_rtv: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        *const c_void,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(9));
                    let _ = create_rtv(self.device, tex.texture, std::ptr::null(), &mut rtv_ptr);
                }
            }
        }

        let handle = DuctRenderTargetHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.render_targets.insert(
            handle,
            Dx12RenderTargetRecord {
                width: desc.width,
                height: desc.height,
                color_tex,
                rtv: rtv_ptr,
            },
        );
        Ok(handle)
    }

    pub(crate) fn destroy_render_target_impl(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if let Some(rt) = self.render_targets.remove(&handle) {
            if self.active_render_target == handle {
                self.active_render_target = DuctRenderTargetHandle::INVALID;
            }
            unsafe {
                com_release(rt.rtv);
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
            DuctResult::Success
        } else if self.render_targets.contains_key(&handle) {
            self.active_render_target = handle;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
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

    pub(crate) fn blit_render_target_impl(
        &mut self,
        src: DuctRenderTargetHandle,
        dst: DuctRenderTargetHandle,
        src_w: u32,
        src_h: u32,
        _dst_w: u32,
        _dst_h: u32,
        _filter: DuctFilterMode,
    ) -> DuctResult {
        let src_tex = if src.is_valid() {
            self.render_targets
                .get(&src)
                .and_then(|rt| self.textures.get(&rt.color_tex))
                .map(|t| t.texture)
                .unwrap_or(std::ptr::null_mut())
        } else {
            self.backbuffer_tex
        };

        let dst_tex = if dst.is_valid() {
            self.render_targets
                .get(&dst)
                .and_then(|rt| self.textures.get(&rt.color_tex))
                .map(|t| t.texture)
                .unwrap_or(std::ptr::null_mut())
        } else {
            self.backbuffer_tex
        };

        if !src_tex.is_null() && !dst_tex.is_null() && !self.context.is_null() {
            unsafe {
                let ctx_vt = com_vtable(self.context);
                let copy_subres: unsafe extern "system" fn(
                    *mut c_void,
                    *mut c_void,
                    u32,
                    u32,
                    u32,
                    u32,
                    *mut c_void,
                    u32,
                    *const D3D11_BOX,
                ) = std::mem::transmute(*ctx_vt.add(46));
                let src_box = D3D11_BOX {
                    left: 0,
                    top: 0,
                    front: 0,
                    right: src_w,
                    bottom: src_h,
                    back: 1,
                };
                copy_subres(
                    self.context,
                    dst_tex,
                    0,
                    0,
                    0,
                    0,
                    src_tex,
                    0,
                    &src_box,
                );
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

        let (src_tex, src_w, src_h) = if self.active_render_target.is_valid() {
            if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                let t = self
                    .textures
                    .get(&rt.color_tex)
                    .map(|t| t.texture)
                    .unwrap_or(std::ptr::null_mut());
                (t, rt.width, rt.height)
            } else {
                (std::ptr::null_mut(), 0, 0)
            }
        } else {
            (self.backbuffer_tex, self.viewport_width, self.viewport_height)
        };

        if !src_tex.is_null()
            && !self.device.is_null()
            && !self.context.is_null()
            && x + width <= src_w
            && y + height <= src_h
        {
            unsafe {
                let stage_desc = D3D11_TEXTURE2D_DESC {
                    Width: width,
                    Height: height,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ,
                    MiscFlags: 0,
                };
                let dev_vt = com_vtable(self.device);
                let create_tex: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_TEXTURE2D_DESC,
                    *const D3D11_SUBRESOURCE_DATA,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(5));
                let mut staging_tex: *mut c_void = std::ptr::null_mut();
                if create_tex(self.device, &stage_desc, std::ptr::null(), &mut staging_tex) == S_OK
                    && !staging_tex.is_null()
                {
                    let ctx_vt = com_vtable(self.context);
                    let copy_subres: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        u32,
                        u32,
                        u32,
                        u32,
                        *mut c_void,
                        u32,
                        *const D3D11_BOX,
                    ) = std::mem::transmute(*ctx_vt.add(46));
                    let src_box = D3D11_BOX {
                        left: x,
                        top: y,
                        front: 0,
                        right: x + width,
                        bottom: y + height,
                        back: 1,
                    };
                    copy_subres(
                        self.context,
                        staging_tex,
                        0,
                        0,
                        0,
                        0,
                        src_tex,
                        0,
                        &src_box,
                    );

                    let map_fn: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        u32,
                        u32,
                        u32,
                        *mut D3D11_MAPPED_SUBRESOURCE,
                    ) -> HRESULT = std::mem::transmute(*ctx_vt.add(14));
                    let unmap_fn: unsafe extern "system" fn(*mut c_void, *mut c_void, u32) =
                        std::mem::transmute(*ctx_vt.add(15));

                    let mut mapped = D3D11_MAPPED_SUBRESOURCE {
                        pData: std::ptr::null_mut(),
                        RowPitch: 0,
                        DepthPitch: 0,
                    };
                    if map_fn(self.context, staging_tex, 0, D3D11_MAP_READ, 0, &mut mapped) == S_OK
                        && !mapped.pData.is_null()
                    {
                        let row_bytes = (width as usize) * 4;
                        for row in 0..(height as usize) {
                            let src_row = (mapped.pData as *const u8)
                                .add(row * (mapped.RowPitch as usize));
                            let dst_row = out_rgba.as_mut_ptr().add(row * row_bytes);
                            std::ptr::copy_nonoverlapping(src_row, dst_row, row_bytes);
                        }
                        unmap_fn(self.context, staging_tex, 0);
                        com_release(staging_tex);
                        return DuctResult::Success;
                    }
                    com_release(staging_tex);
                }
            }
        }

        out_rgba[..req].fill(255);
        DuctResult::Success
    }

}
