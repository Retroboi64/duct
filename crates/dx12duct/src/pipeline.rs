use duct_core::*;
use std::ffi::{c_char, c_void};
use crate::types::*;

impl Dx12Pipeline {
    pub(crate) unsafe fn compile_hlsl(&self, src: &[u8], entry: &[u8], target: &[u8]) -> Option<Vec<u8>> {
        let loader = self.loader.as_ref()?;
        let compile_fn = loader.d3d_compile?;
        let mut code_blob: *mut c_void = std::ptr::null_mut();
        let mut err_blob: *mut c_void = std::ptr::null_mut();
        let hr = compile_fn(
            src.as_ptr() as *const c_void,
            src.len(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            entry.as_ptr() as *const c_char,
            target.as_ptr() as *const c_char,
            0,
            0,
            &mut code_blob,
            &mut err_blob,
        );
        if hr != S_OK && !err_blob.is_null() {
            let ptr = blob_pointer(err_blob);
            let len = blob_size(err_blob);
            let err_str = String::from_utf8_lossy(std::slice::from_raw_parts(ptr, len));
            eprintln!("[dx12duct][ERROR] D3DCompile failed:\n{}", err_str);
        }
        if !err_blob.is_null() {
            com_release(err_blob);
        }
        if hr == S_OK && !code_blob.is_null() {
            let ptr = blob_pointer(code_blob);
            let len = blob_size(code_blob);
            let bytes = std::slice::from_raw_parts(ptr, len).to_vec();
            com_release(code_blob);
            Some(bytes)
        } else {
            None
        }
    }

    pub(crate) unsafe fn create_vs(&self, bytecode: &[u8]) -> *mut c_void {
        if self.device.is_null() || bytecode.is_empty() {
            return std::ptr::null_mut();
        }
        let vt = com_vtable(self.device);
        let create_vs_fn: unsafe extern "system" fn(
            *mut c_void,
            *const c_void,
            usize,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(*vt.add(12));
        let mut vs: *mut c_void = std::ptr::null_mut();
        let _ = create_vs_fn(
            self.device,
            bytecode.as_ptr() as *const c_void,
            bytecode.len(),
            std::ptr::null_mut(),
            &mut vs,
        );
        vs
    }

    pub(crate) unsafe fn create_ps(&self, bytecode: &[u8]) -> *mut c_void {
        if self.device.is_null() || bytecode.is_empty() {
            return std::ptr::null_mut();
        }
        let vt = com_vtable(self.device);
        let create_ps_fn: unsafe extern "system" fn(
            *mut c_void,
            *const c_void,
            usize,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(*vt.add(15));
        let mut ps: *mut c_void = std::ptr::null_mut();
        let _ = create_ps_fn(
            self.device,
            bytecode.as_ptr() as *const c_void,
            bytecode.len(),
            std::ptr::null_mut(),
            &mut ps,
        );
        ps
    }

    pub(crate) unsafe fn create_cs(&self, bytecode: &[u8]) -> *mut c_void {
        if self.device.is_null() || bytecode.is_empty() {
            return std::ptr::null_mut();
        }
        let vt = com_vtable(self.device);
        let create_cs_fn: unsafe extern "system" fn(
            *mut c_void,
            *const c_void,
            usize,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(*vt.add(18));
        let mut cs: *mut c_void = std::ptr::null_mut();
        let _ = create_cs_fn(
            self.device,
            bytecode.as_ptr() as *const c_void,
            bytecode.len(),
            std::ptr::null_mut(),
            &mut cs,
        );
        cs
    }

    pub(crate) fn create_shader_impl(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return Err(DuctResult::ErrorInvalidParameter);
        }
        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let src_str = std::str::from_utf8(slice).unwrap_or("");
        let is_glsl = src_str.contains("#version") || src_str.contains("gl_Position") || src_str.contains("precision ");

        let hlsl_code: Vec<u8> = if is_glsl {
            match duct_core::shader::glsl_to_hlsl(src_str, desc.stage) {
                Ok(hlsl_str) => hlsl_str.into_bytes(),
                Err(err) => {
                    eprintln!("[dx12duct][ERROR] GLSL to HLSL cross-compilation failed:\n{}", err);
                    return Err(DuctResult::ErrorInvalidParameter);
                }
            }
        } else {
            slice.to_vec()
        };

        let mut shader_obj_color = std::ptr::null_mut();
        let mut bytecode_color = Vec::new();

        let (target, is_vs, is_ps, is_cs) = match desc.stage {
            DuctShaderStage::Vertex => (b"vs_5_0\0".as_slice(), true, false, false),
            DuctShaderStage::Fragment => (b"ps_5_0\0".as_slice(), false, true, false),
            DuctShaderStage::Compute => (b"cs_5_0\0".as_slice(), false, false, true),
        };

        let (bytecode, shader_obj) = unsafe {
            if let Some(bc) = self.compile_hlsl(&hlsl_code, b"main\0", target) {
                let obj = if is_vs {
                    self.create_vs(&bc)
                } else if is_ps {
                    self.create_ps(&bc)
                } else if is_cs {
                    self.create_cs(&bc)
                } else {
                    std::ptr::null_mut()
                };
                (bc, obj)
            } else {
                eprintln!("[dx12duct][ERROR] Failed to compile HLSL shader for {:?}", desc.stage);
                return Err(DuctResult::ErrorInvalidParameter);
            }
        };

        unsafe {
            if is_vs {
                if let Some(bc_col) = self.compile_hlsl(BUILTIN_VS_COLOR_HLSL, b"main\0", b"vs_5_0\0") {
                    shader_obj_color = self.create_vs(&bc_col);
                    bytecode_color = bc_col;
                }
            }
        }

        let handle = DuctShaderHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.shaders.insert(
            handle,
            Dx12ShaderRecord {
                stage: desc.stage,
                shader_obj,
                shader_obj_color,
                bytecode,
                bytecode_color,
            },
        );
        Ok(handle)
    }

    pub(crate) fn reload_shader_impl(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        if !self.shaders.contains_key(&handle) {
            return DuctResult::ErrorResourceNotFound;
        }
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return DuctResult::ErrorInvalidParameter;
        }
        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let src_str = std::str::from_utf8(slice).unwrap_or("");
        let is_glsl = src_str.contains("#version") || src_str.contains("gl_Position") || src_str.contains("precision ");

        let hlsl_code: Vec<u8> = if is_glsl {
            match duct_core::shader::glsl_to_hlsl(src_str, desc.stage) {
                Ok(hlsl_str) => hlsl_str.into_bytes(),
                Err(err) => {
                    eprintln!("[dx12duct][ERROR] Shader reload GLSL cross-compilation failed:\n{}", err);
                    return DuctResult::ErrorInvalidParameter;
                }
            }
        } else {
            slice.to_vec()
        };

        let (target, is_vs, is_ps, is_cs) = match desc.stage {
            DuctShaderStage::Vertex => (b"vs_5_0\0".as_slice(), true, false, false),
            DuctShaderStage::Fragment => (b"ps_5_0\0".as_slice(), false, true, false),
            DuctShaderStage::Compute => (b"cs_5_0\0".as_slice(), false, false, true),
        };

        unsafe {
            let (new_bc, new_obj) = if let Some(bc) = self.compile_hlsl(&hlsl_code, b"main\0", target) {
                let obj = if is_vs {
                    self.create_vs(&bc)
                } else if is_ps {
                    self.create_ps(&bc)
                } else if is_cs {
                    self.create_cs(&bc)
                } else {
                    std::ptr::null_mut()
                };
                (bc, obj)
            } else {
                eprintln!("[dx12duct][ERROR] Shader reload HLSL compile failed for {:?}", desc.stage);
                return DuctResult::ErrorInvalidParameter;
            };

            if let Some(rec) = self.shaders.get_mut(&handle) {
                com_release(rec.shader_obj);
                rec.shader_obj = new_obj;
                rec.bytecode = new_bc;
                rec.stage = desc.stage;
            }
        }
        println!("[dx12duct] Shader {:?} hot-reloaded on D3D12 device.", handle);
        DuctResult::Success
    }

    pub(crate) fn destroy_shader_impl(&mut self, handle: DuctShaderHandle) -> DuctResult {
        if let Some(rec) = self.shaders.remove(&handle) {
            unsafe {
                com_release(rec.shader_obj);
                com_release(rec.shader_obj_color);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    pub(crate) fn set_pipeline_state_impl(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult {
        self.pipeline_switches_in_frame += 1;
        self.current_pso = Some(*desc);

        if !self.device.is_null() && !self.context.is_null() {
            unsafe {
                let dev_vt = com_vtable(self.device);
                let ctx_vt = com_vtable(self.context);

                let cull_mode = match desc.cull_mode {
                    DuctCullMode::None => D3D11_CULL_NONE,
                    DuctCullMode::Front => D3D11_CULL_FRONT,
                    DuctCullMode::Back => D3D11_CULL_BACK,
                };
                let rast_desc = D3D11_RASTERIZER_DESC {
                    FillMode: if self.wireframe {
                        D3D11_FILL_WIREFRAME
                    } else {
                        D3D11_FILL_SOLID
                    },
                    CullMode: cull_mode,
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
                    let rs_set: unsafe extern "system" fn(*mut c_void, *mut c_void) =
                        std::mem::transmute(*ctx_vt.add(43));
                    rs_set(self.context, rs);
                    com_release(rs);
                }

                let rt_blend = match desc.blend_mode {
                    DuctBlendMode::Opaque => D3D11_RENDER_TARGET_BLEND_DESC {
                        BlendEnable: 0,
                        SrcBlend: D3D11_BLEND_ONE,
                        DestBlend: D3D11_BLEND_ZERO,
                        BlendOp: D3D11_BLEND_OP_ADD,
                        SrcBlendAlpha: D3D11_BLEND_ONE,
                        DestBlendAlpha: D3D11_BLEND_ZERO,
                        BlendOpAlpha: D3D11_BLEND_OP_ADD,
                        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL,
                    },
                    DuctBlendMode::AlphaBlend => D3D11_RENDER_TARGET_BLEND_DESC {
                        BlendEnable: 1,
                        SrcBlend: D3D11_BLEND_SRC_ALPHA,
                        DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                        BlendOp: D3D11_BLEND_OP_ADD,
                        SrcBlendAlpha: D3D11_BLEND_ONE,
                        DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                        BlendOpAlpha: D3D11_BLEND_OP_ADD,
                        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL,
                    },
                    DuctBlendMode::Additive => D3D11_RENDER_TARGET_BLEND_DESC {
                        BlendEnable: 1,
                        SrcBlend: D3D11_BLEND_SRC_ALPHA,
                        DestBlend: D3D11_BLEND_ONE,
                        BlendOp: D3D11_BLEND_OP_ADD,
                        SrcBlendAlpha: D3D11_BLEND_ONE,
                        DestBlendAlpha: D3D11_BLEND_ONE,
                        BlendOpAlpha: D3D11_BLEND_OP_ADD,
                        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL,
                    },
                };
                let blend_desc = D3D11_BLEND_DESC {
                    AlphaToCoverageEnable: 0,
                    IndependentBlendEnable: 0,
                    RenderTarget: [rt_blend; 8],
                };
                let create_bs: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_BLEND_DESC,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(20));
                let mut bs: *mut c_void = std::ptr::null_mut();
                if create_bs(self.device, &blend_desc, &mut bs) == S_OK && !bs.is_null() {
                    let om_set_bs: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        *const [f32; 4],
                        u32,
                    ) = std::mem::transmute(*ctx_vt.add(35));
                    let factor = [0.0f32; 4];
                    om_set_bs(self.context, bs, &factor, 0xffff_ffff);
                    com_release(bs);
                }
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_wireframe_impl(&mut self, enabled: bool) -> DuctResult {
        self.wireframe = enabled;
        if let Some(pso) = self.current_pso {
            let _ = self.set_pipeline_state(&pso);
        }
        DuctResult::Success
    }

}
