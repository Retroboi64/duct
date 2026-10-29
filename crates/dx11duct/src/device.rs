use duct_core::*;
use std::ffi::{c_char, c_void, CString};
use crate::types::*;

impl Dx11Pipeline {
    pub(crate) unsafe fn create_swapchain_rtv(&mut self) {
        if self.swapchain.is_null() || self.device.is_null() {
            return;
        }
        let sc_vt = com_vtable(self.swapchain);
        let get_buffer: unsafe extern "system" fn(
            *mut c_void,
            u32,
            *const GUID,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(*sc_vt.add(9));
        let mut backbuf: *mut c_void = std::ptr::null_mut();
        if get_buffer(self.swapchain, 0, &IID_ID3D11Texture2D, &mut backbuf) == S_OK
            && !backbuf.is_null()
        {
            self.backbuffer_tex = backbuf;
            let dev_vt = com_vtable(self.device);
            let create_rtv: unsafe extern "system" fn(
                *mut c_void,
                *mut c_void,
                *const c_void,
                *mut *mut c_void,
            ) -> HRESULT = std::mem::transmute(*dev_vt.add(9));
            let mut rtv: *mut c_void = std::ptr::null_mut();
            if create_rtv(self.device, backbuf, std::ptr::null(), &mut rtv) == S_OK {
                self.backbuffer_rtv = rtv;
            }
        }
    }

    pub(crate) fn init_impl(&mut self, config: &DuctInitConfig) -> DuctResult {
        if !config.window.is_valid() {
            eprintln!("[dx11duct][ERROR] Pipeline initialization failed: an attached native window is required.");
            return DuctResult::ErrorWindowRequired;
        }

        self.viewport_width = config.width.max(1);
        self.viewport_height = config.height.max(1);
        self.vsync_enabled = config.vsync;

        println!("[dx11duct] Initializing DirectX 11 Hardware Pipeline...");

        #[cfg(target_os = "windows")]
        unsafe {
            if let Some(loader) = D3D11Loader::new() {
                let feature_levels = [
                    D3D_FEATURE_LEVEL_11_1,
                    D3D_FEATURE_LEVEL_11_0,
                    D3D_FEATURE_LEVEL_10_1,
                    D3D_FEATURE_LEVEL_10_0,
                ];
                let mut chosen_fl = D3D_FEATURE_LEVEL_11_0;
                let hwnd = config.window.native_window;
                let is_real_window = IsWindow(hwnd) != 0;

                if is_real_window {
                    if let Some(create_fn) = loader.create_device_and_swapchain {
                        let sc_desc = DXGI_SWAP_CHAIN_DESC {
                            BufferDesc: DXGI_MODE_DESC {
                                Width: self.viewport_width,
                                Height: self.viewport_height,
                                RefreshRate: DXGI_RATIONAL {
                                    Numerator: 60,
                                    Denominator: 1,
                                },
                                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                                ScanlineOrdering: 0,
                                Scaling: 0,
                            },
                            SampleDesc: DXGI_SAMPLE_DESC {
                                Count: 1,
                                Quality: 0,
                            },
                            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                            BufferCount: 2,
                            OutputWindow: hwnd,
                            Windowed: 1,
                            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
                            Flags: 0,
                        };
                        let mut hr = create_fn(
                            std::ptr::null_mut(),
                            D3D_DRIVER_TYPE_HARDWARE,
                            std::ptr::null_mut(),
                            0,
                            feature_levels.as_ptr(),
                            feature_levels.len() as u32,
                            D3D11_SDK_VERSION,
                            &sc_desc,
                            &mut self.swapchain,
                            &mut self.device,
                            &mut chosen_fl,
                            &mut self.context,
                        );
                        if hr != S_OK {
                            hr = create_fn(
                                std::ptr::null_mut(),
                                D3D_DRIVER_TYPE_WARP,
                                std::ptr::null_mut(),
                                0,
                                feature_levels.as_ptr(),
                                feature_levels.len() as u32,
                                D3D11_SDK_VERSION,
                                &sc_desc,
                                &mut self.swapchain,
                                &mut self.device,
                                &mut chosen_fl,
                                &mut self.context,
                            );
                        }
                        if hr == S_OK {
                            self.create_swapchain_rtv();
                        }
                    }
                }

                // Fallback to headless D3D11 device when given a mock HWND (e.g. unit/FFI tests)
                if self.device.is_null() {
                    if let Some(create_dev_fn) = loader.create_device {
                        let _ = create_dev_fn(
                            std::ptr::null_mut(),
                            D3D_DRIVER_TYPE_HARDWARE,
                            std::ptr::null_mut(),
                            0,
                            feature_levels.as_ptr(),
                            feature_levels.len() as u32,
                            D3D11_SDK_VERSION,
                            &mut self.device,
                            &mut chosen_fl,
                            &mut self.context,
                        );
                    }
                }

                self.loader = Some(loader);

                if !self.device.is_null() && !self.context.is_null() {
                    // Query GPU Adapter Name via IDXGIDevice -> IDXGIAdapter
                    let mut dxgi_dev: *mut c_void = std::ptr::null_mut();
                    if com_query_interface(self.device, &IID_IDXGIDevice, &mut dxgi_dev) == S_OK
                        && !dxgi_dev.is_null()
                    {
                        let dd_vt = com_vtable(dxgi_dev);
                        let get_adapter: unsafe extern "system" fn(
                            *mut c_void,
                            *mut *mut c_void,
                        ) -> HRESULT = std::mem::transmute(*dd_vt.add(7));
                        let mut adapter: *mut c_void = std::ptr::null_mut();
                        if get_adapter(dxgi_dev, &mut adapter) == S_OK && !adapter.is_null() {
                            let ad_vt = com_vtable(adapter);
                            let get_desc: unsafe extern "system" fn(
                                *mut c_void,
                                *mut DXGI_ADAPTER_DESC,
                            ) -> HRESULT = std::mem::transmute(*ad_vt.add(8));
                            let mut desc = DXGI_ADAPTER_DESC::default();
                            if get_desc(adapter, &mut desc) == S_OK {
                                let end = desc
                                    .Description
                                    .iter()
                                    .position(|&c| c == 0)
                                    .unwrap_or(desc.Description.len());
                                let gpu_name = String::from_utf16_lossy(&desc.Description[..end]);
                                self.name = format!("DirectX 11 ({})", gpu_name);
                                self.extra_info =
                                    CString::new(format!("D3D11.0 | {}", gpu_name))
                                        .unwrap_or_default();
                            }
                            com_release(adapter);
                        }
                        com_release(dxgi_dev);
                    }

                    // Compile built-in default shaders and input layouts
                    if let Some(vs_bc) = self.compile_hlsl(BUILTIN_VS_HLSL, b"main\0", b"vs_5_0\0")
                    {
                        self.default_vs = self.create_vs(&vs_bc);
                        let dev_vt = com_vtable(self.device);
                        let create_il: unsafe extern "system" fn(
                            *mut c_void,
                            *const D3D11_INPUT_ELEMENT_DESC,
                            u32,
                            *const c_void,
                            usize,
                            *mut *mut c_void,
                        ) -> HRESULT = std::mem::transmute(*dev_vt.add(11));
                        let elem = D3D11_INPUT_ELEMENT_DESC {
                            SemanticName: b"POSITION\0".as_ptr() as *const c_char,
                            SemanticIndex: 0,
                            Format: DXGI_FORMAT_R32G32B32_FLOAT,
                            InputSlot: 0,
                            AlignedByteOffset: 0,
                            InputSlotClass: 0,
                            InstanceDataStepRate: 0,
                        };
                        let _ = create_il(
                            self.device,
                            &elem,
                            1,
                            vs_bc.as_ptr() as *const c_void,
                            vs_bc.len(),
                            &mut self.default_input_layout,
                        );
                    }

                    if let Some(vs_col_bc) =
                        self.compile_hlsl(BUILTIN_VS_COLOR_HLSL, b"main\0", b"vs_5_0\0")
                    {
                        self.default_vs_color = self.create_vs(&vs_col_bc);
                        let dev_vt = com_vtable(self.device);
                        let create_il: unsafe extern "system" fn(
                            *mut c_void,
                            *const D3D11_INPUT_ELEMENT_DESC,
                            u32,
                            *const c_void,
                            usize,
                            *mut *mut c_void,
                        ) -> HRESULT = std::mem::transmute(*dev_vt.add(11));
                        let elems = [
                            D3D11_INPUT_ELEMENT_DESC {
                                SemanticName: b"POSITION\0".as_ptr() as *const c_char,
                                SemanticIndex: 0,
                                Format: DXGI_FORMAT_R32G32B32_FLOAT,
                                InputSlot: 0,
                                AlignedByteOffset: 0,
                                InputSlotClass: 0,
                                InstanceDataStepRate: 0,
                            },
                            D3D11_INPUT_ELEMENT_DESC {
                                SemanticName: b"COLOR\0".as_ptr() as *const c_char,
                                SemanticIndex: 0,
                                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                                InputSlot: 0,
                                AlignedByteOffset: 12,
                                InputSlotClass: 0,
                                InstanceDataStepRate: 0,
                            },
                        ];
                        let _ = create_il(
                            self.device,
                            elems.as_ptr(),
                            2,
                            vs_col_bc.as_ptr() as *const c_void,
                            vs_col_bc.len(),
                            &mut self.default_color_input_layout,
                        );
                    }

                    if let Some(ps_bc) = self.compile_hlsl(BUILTIN_PS_HLSL, b"main\0", b"ps_5_0\0")
                    {
                        self.default_ps = self.create_ps(&ps_bc);
                    }

                    // Create 96-byte Constant Buffer (b0)
                    let cb_desc = D3D11_BUFFER_DESC {
                        ByteWidth: std::mem::size_of::<Dx11ConstantBlock>() as u32,
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                        CPUAccessFlags: 0,
                        MiscFlags: 0,
                        StructureByteStride: 0,
                    };
                    let init_data = D3D11_SUBRESOURCE_DATA {
                        pSysMem: &self.constants as *const Dx11ConstantBlock as *const c_void,
                        SysMemPitch: 0,
                        SysMemSlicePitch: 0,
                    };
                    let dev_vt = com_vtable(self.device);
                    let create_buf: unsafe extern "system" fn(
                        *mut c_void,
                        *const D3D11_BUFFER_DESC,
                        *const D3D11_SUBRESOURCE_DATA,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(3));
                    let _ = create_buf(self.device, &cb_desc, &init_data, &mut self.constant_buffer);

                    // Configure default Rasterizer State (CullMode::None, ScissorEnable=TRUE)
                    let rast_desc = D3D11_RASTERIZER_DESC {
                        FillMode: D3D11_FILL_SOLID,
                        CullMode: D3D11_CULL_NONE,
                        FrontCounterClockwise: 1,
                        DepthBias: 0,
                        DepthBiasClamp: 0.0,
                        SlopeScaledDepthBias: 0.0,
                        DepthClipEnable: 1,
                        ScissorEnable: 1,
                        MultisampleEnable: 0,
                        AntialiasedLineEnable: 0,
                    };
                    let create_rs: unsafe extern "system" fn(
                        *mut c_void,
                        *const D3D11_RASTERIZER_DESC,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(22));
                    let mut rs: *mut c_void = std::ptr::null_mut();
                    if create_rs(self.device, &rast_desc, &mut rs) == S_OK && !rs.is_null() {
                        let ctx_vt = com_vtable(self.context);
                        let rs_set: unsafe extern "system" fn(*mut c_void, *mut c_void) =
                            std::mem::transmute(*ctx_vt.add(43));
                        rs_set(self.context, rs);
                        com_release(rs);
                    }

                    println!(
                        "[dx11duct] D3D11 Hardware Device initialized: {} (SwapchainActive: {})",
                        self.name,
                        !self.swapchain.is_null()
                    );
                }
            }
        }

        self.initialized = true;
        DuctResult::Success
    }

    pub(crate) fn shutdown_impl(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::Success;
        }
        println!(
            "[dx11duct] Releasing D3D11 resources and shutting down device (Total Frames: {}).",
            self.frame_count
        );

        unsafe {
            for (_, s) in self.shaders.drain() {
                com_release(s.shader_obj);
                com_release(s.shader_obj_color);
            }
            for (_, b) in self.buffers.drain() {
                com_release(b.buffer);
            }
            for (_, rt) in self.render_targets.drain() {
                com_release(rt.rtv);
            }
            for (_, t) in self.textures.drain() {
                com_release(t.sampler);
                com_release(t.srv);
                com_release(t.texture);
            }
            for (_, rs) in self.cached_rasterizer_states.drain() {
                com_release(rs);
            }
            for (_, bs) in self.cached_blend_states.drain() {
                com_release(bs);
            }
            for (_, ds) in self.cached_depth_stencil_states.drain() {
                com_release(ds);
            }
            com_release(self.constant_buffer);
            self.constant_buffer = std::ptr::null_mut();
            com_release(self.default_input_layout);
            self.default_input_layout = std::ptr::null_mut();
            com_release(self.default_color_input_layout);
            self.default_color_input_layout = std::ptr::null_mut();
            com_release(self.default_vs);
            self.default_vs = std::ptr::null_mut();
            com_release(self.default_vs_color);
            self.default_vs_color = std::ptr::null_mut();
            com_release(self.default_ps);
            self.default_ps = std::ptr::null_mut();
            com_release(self.backbuffer_rtv);
            self.backbuffer_rtv = std::ptr::null_mut();
            com_release(self.backbuffer_tex);
            self.backbuffer_tex = std::ptr::null_mut();
            com_release(self.swapchain);
            self.swapchain = std::ptr::null_mut();
            com_release(self.context);
            self.context = std::ptr::null_mut();
            com_release(self.device);
            self.device = std::ptr::null_mut();
        }

        self.loader = None;
        self.initialized = false;
        DuctResult::Success
    }

    pub(crate) fn resize_viewport_impl(&mut self, width: u32, height: u32) -> DuctResult {
        if width == 0 || height == 0 {
            return DuctResult::Success;
        }
        self.viewport_width = width;
        self.viewport_height = height;

        unsafe {
            if !self.swapchain.is_null() && !self.context.is_null() {
                let ctx_vt = com_vtable(self.context);
                let om_set_rt: unsafe extern "system" fn(
                    *mut c_void,
                    u32,
                    *const *mut c_void,
                    *mut c_void,
                ) = std::mem::transmute(*ctx_vt.add(33));
                om_set_rt(self.context, 0, std::ptr::null(), std::ptr::null_mut());

                com_release(self.backbuffer_rtv);
                self.backbuffer_rtv = std::ptr::null_mut();
                com_release(self.backbuffer_tex);
                self.backbuffer_tex = std::ptr::null_mut();

                let sc_vt = com_vtable(self.swapchain);
                let resize_buffers: unsafe extern "system" fn(
                    *mut c_void,
                    u32,
                    u32,
                    u32,
                    u32,
                    u32,
                ) -> HRESULT = std::mem::transmute(*sc_vt.add(13));
                let _ = resize_buffers(
                    self.swapchain,
                    0,
                    width,
                    height,
                    DXGI_FORMAT_R8G8B8A8_UNORM,
                    0,
                );
                self.create_swapchain_rtv();
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_vsync_impl(&mut self, enabled: bool) -> DuctResult {
        self.vsync_enabled = enabled;
        DuctResult::Success
    }

}
