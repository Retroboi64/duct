//! DirectX 12 backend pipeline stub for Project32 duct graphics subsystem.
//! Compiles into `duct.dx12.dll` (or platform equivalent).

use duct_core::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};

pub struct Dx12Pipeline {
    name: String,
    version: String,
    extra_info: CString,
    initialized: bool,
    viewport_width: u32,
    viewport_height: u32,
    frame_in_progress: bool,
    frame_count: u64,
    draw_calls: u32,
    next_handle_id: u64,
    shaders: HashMap<DuctShaderHandle, DuctShaderStage>,
    buffers: HashMap<DuctBufferHandle, usize>,
    textures: HashMap<DuctTextureHandle, (u32, u32)>,
    device_mock_ptr: *mut c_void,
    command_queue_mock_ptr: *mut c_void,
}

impl Default for Dx12Pipeline {
    fn default() -> Self {
        Self {
            name: "DirectX 12 (D3D12 Feature Level 12_1)".to_string(),
            version: "0.1.0".to_string(),
            extra_info: CString::new("DirectX 12.0 Agility SDK - Project32 D3D12 Stub").unwrap(),
            initialized: false,
            viewport_width: 800,
            viewport_height: 600,
            frame_in_progress: false,
            frame_count: 0,
            draw_calls: 0,
            next_handle_id: 1,
            shaders: HashMap::new(),
            buffers: HashMap::new(),
            textures: HashMap::new(),
            device_mock_ptr: (0xD3D12_0001_usize) as *mut c_void,
            command_queue_mock_ptr: (0xD3D12_0002_usize) as *mut c_void,
        }
    }
}

unsafe extern "C" fn dx12_get_proc_address(_name: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}

impl RenderPipeline for Dx12Pipeline {
    fn backend_type(&self) -> DuctBackendType {
        DuctBackendType::DirectX12
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn init(&mut self, config: &DuctInitConfig) -> DuctResult {
        if !config.window.is_valid() {
            eprintln!("[dx12duct][ERROR] Pipeline initialization failed: an attached native window is required.");
            return DuctResult::ErrorWindowRequired;
        }

        #[cfg(not(target_os = "windows"))]
        {
            eprintln!("[dx12duct][ERROR] DirectX 12 is only supported on Windows operating systems.");
            return DuctResult::ErrorUnsupported;
        }

        self.viewport_width = config.width;
        self.viewport_height = config.height;
        self.initialized = true;

        println!(
            "[dx12duct] Initializing DirectX 12 Pipeline (D3D12 Feature Level 12_1)..."
        );
        println!(
            "[dx12duct] D3D12CreateDevice and CommandQueue initialized (Viewport: {}x{}, VSync: {}, Window: {:?}).",
            self.viewport_width, self.viewport_height, config.vsync, config.window.native_window
        );

        DuctResult::Success
    }

    fn shutdown(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::Success;
        }
        println!(
            "[dx12duct] Flushing CommandQueue and shutting down D3D12 device (Total Frames: {}).",
            self.frame_count
        );
        self.shaders.clear();
        self.buffers.clear();
        self.textures.clear();
        self.initialized = false;
        DuctResult::Success
    }

    fn resize_viewport(&mut self, width: u32, height: u32) -> DuctResult {
        self.viewport_width = width;
        self.viewport_height = height;
        println!(
            "[dx12duct] ResizeBuffers & RSSetViewports: D3D12_VIEWPORT(0, 0, {}, {})",
            width, height
        );
        DuctResult::Success
    }

    fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
        println!("[dx12duct] RSSetScissorRects: [{}, {}, {}, {}]", x, y, width, height);
        DuctResult::Success
    }

    fn begin_frame(&mut self, clear_color: DuctColor) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }
        self.frame_in_progress = true;
        self.draw_calls = 0;
        println!(
            "[dx12duct] Frame begun - ID3D12GraphicsCommandList::ClearRenderTargetView(RGBA=[{:.2}, {:.2}, {:.2}, {:.2}])",
            clear_color.r, clear_color.g, clear_color.b, clear_color.a
        );
        DuctResult::Success
    }

    fn end_frame(&mut self) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.frame_in_progress = false;
        self.frame_count += 1;
        println!("[dx12duct] ID3D12GraphicsCommandList::Close & ExecuteCommandLists()");
        DuctResult::Success
    }

    fn present(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }
        println!(
            "[dx12duct] IDXGISwapChain::Present(SyncInterval=1) - Frame #{} finished (Draw Calls: {})",
            self.frame_count, self.draw_calls
        );
        DuctResult::Success
    }

    fn create_shader(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        let handle = DuctShaderHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.shaders.insert(handle, desc.stage);
        println!(
            "[dx12duct] D3D12 Bytecode Shader compiled -> Handle ID: {} ({:?})",
            handle.0, desc.stage
        );
        Ok(handle)
    }

    fn reload_shader(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        if self.shaders.contains_key(&handle) {
            self.shaders.insert(handle, desc.stage);
            println!("[dx12duct] D3D12 Shader reloaded -> Handle ID: {} ({:?})", handle.0, desc.stage);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_shader(&mut self, handle: DuctShaderHandle) -> DuctResult {
        if self.shaders.remove(&handle).is_some() {
            println!("[dx12duct] Shader released -> Handle ID: {}", handle.0);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_buffer(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
        let handle = DuctBufferHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.buffers.insert(handle, desc.size_bytes);
        println!(
            "[dx12duct] ID3D12Device::CreateCommittedResource (Default Heap, {} bytes) -> Handle ID: {}",
            desc.size_bytes, handle.0
        );
        Ok(handle)
    }

    fn update_buffer(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
        if self.buffers.contains_key(&handle) {
            println!("[dx12duct] UpdateSubresources: Buffer {}, Offset {}, Size {} bytes", handle.0, offset, data.len());
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if self.buffers.remove(&handle).is_some() {
            println!("[dx12duct] Buffer resource released -> Handle ID: {}", handle.0);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_vertex_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if self.buffers.contains_key(&handle) {
            println!("[dx12duct] IASetVertexBuffers(Slot=0, Buffer={})", handle.0);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_index_buffer(&mut self, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult {
        if self.buffers.contains_key(&handle) {
            println!("[dx12duct] IASetIndexBuffer(Buffer={}, Format={:?})", handle.0, format);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_uniform_buffer(&mut self, slot: u32, handle: DuctBufferHandle) -> DuctResult {
        if self.buffers.contains_key(&handle) {
            println!("[dx12duct] SetGraphicsRootConstantBufferView(Slot={}, Buffer={})", slot, handle.0);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_texture_2d(
        &mut self,
        width: u32,
        height: u32,
        _format: DuctTextureFormat,
        _data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctResult> {
        let handle = DuctTextureHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.textures.insert(handle, (width, height));
        println!("[dx12duct] ID3D12Device::CreateTexture2D ({}x{}) -> Handle ID: {}", width, height, handle.0);
        Ok(handle)
    }

    fn update_texture_2d(
        &mut self,
        handle: DuctTextureHandle,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        _data: &[u8],
    ) -> DuctResult {
        if self.textures.contains_key(&handle) {
            println!(
                "[dx12duct] CopyTextureRegion(Texture={}, Region=[{}, {}, {}x{}])",
                handle.0, x, y, width, height
            );
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_texture(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if self.textures.remove(&handle).is_some() {
            println!("[dx12duct] Texture released -> Handle ID: {}", handle.0);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_texture(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult {
        println!("[dx12duct] SetGraphicsRootDescriptorTable(Slot={}, Texture={})", slot, handle.0);
        DuctResult::Success
    }

    fn generate_mipmaps(&mut self, handle: DuctTextureHandle) -> DuctResult {
        println!("[dx12duct] GenerateMips compute pass for Texture {}", handle.0);
        DuctResult::Success
    }

    fn set_texture_filter(
        &mut self,
        handle: DuctTextureHandle,
        min: DuctFilterMode,
        mag: DuctFilterMode,
    ) -> DuctResult {
        println!("[dx12duct] D3D12_SAMPLER_DESC: Texture {}, Min={:?}, Mag={:?}", handle.0, min, mag);
        DuctResult::Success
    }

    fn set_texture_wrap(
        &mut self,
        handle: DuctTextureHandle,
        wrap_s: DuctWrapMode,
        wrap_t: DuctWrapMode,
    ) -> DuctResult {
        println!("[dx12duct] D3D12_SAMPLER_DESC: Texture {}, AddressU={:?}, AddressV={:?}", handle.0, wrap_s, wrap_t);
        DuctResult::Success
    }

    fn create_render_target(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        let handle = DuctRenderTargetHandle(self.next_handle_id);
        self.next_handle_id += 1;
        println!(
            "[dx12duct] CreateRenderTargetView & CreateDepthStencilView ({}x{}, Depth={}) -> Handle ID: {}",
            desc.width, desc.height, desc.has_depth, handle.0
        );
        Ok(handle)
    }

    fn destroy_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        println!("[dx12duct] Released RTV/DSV resources -> Handle ID: {}", handle.0);
        DuctResult::Success
    }

    fn bind_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        println!("[dx12duct] OMSetRenderTargets: Target={}", handle.0);
        DuctResult::Success
    }

    fn get_render_target_texture(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        Ok(DuctTextureHandle(handle.0))
    }

    fn blit_render_target(
        &mut self,
        src: DuctRenderTargetHandle,
        dst: DuctRenderTargetHandle,
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        filter: DuctFilterMode,
    ) -> DuctResult {
        println!(
            "[dx12duct] CopyResource / BlitPass (Src={} [{}x{}] -> Dst={} [{}x{}], Filter={:?})",
            src.0, src_w, src_h, dst.0, dst_w, dst_h, filter
        );
        DuctResult::Success
    }

    fn read_pixels(
        &mut self,
        _x: u32,
        _y: u32,
        width: u32,
        height: u32,
        out_rgba: &mut [u8],
    ) -> DuctResult {
        let req = (width as usize) * (height as usize) * 4;
        if out_rgba.len() < req {
            return DuctResult::ErrorInvalidParameter;
        }
        out_rgba[..req].fill(255);
        DuctResult::Success
    }

    fn set_pipeline_state(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult {
        println!(
            "[dx12duct] SetPipelineState(VS={}, FS={}, Blend={:?}, Cull={:?})",
            desc.vertex_shader.0, desc.fragment_shader.0, desc.blend_mode, desc.cull_mode
        );
        DuctResult::Success
    }

    fn set_wireframe(&mut self, enabled: bool) -> DuctResult {
        println!("[dx12duct] SetFillMode: {}", if enabled { "WIREFRAME" } else { "SOLID" });
        DuctResult::Success
    }

    fn set_vsync(&mut self, enabled: bool) -> DuctResult {
        println!("[dx12duct] IDXGISwapChain SyncInterval = {}", if enabled { 1 } else { 0 });
        DuctResult::Success
    }

    fn set_uniform_int(&mut self, name: &str, val: i32) -> DuctResult {
        println!("[dx12duct] SetConstantBufferInt: {} = {}", name, val);
        DuctResult::Success
    }

    fn set_uniform_float(&mut self, name: &str, val: f32) -> DuctResult {
        println!("[dx12duct] SetConstantBufferValue: {} = {}", name, val);
        DuctResult::Success
    }

    fn set_uniform_vec2(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        println!("[dx12duct] SetConstantBufferValue: {} = [{}, {}]", name, x, y);
        DuctResult::Success
    }

    fn set_uniform_vec3(&mut self, name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        println!("[dx12duct] SetConstantBufferValue: {} = [{}, {}, {}]", name, x, y, z);
        DuctResult::Success
    }

    fn set_uniform_vec4(&mut self, name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        println!("[dx12duct] SetConstantBufferValue: {} = [{}, {}, {}, {}]", name, x, y, z, w);
        DuctResult::Success
    }

    fn set_uniform_mat4(&mut self, name: &str, _elements: &[f32; 16]) -> DuctResult {
        println!("[dx12duct] SetConstantBufferMatrix4x4: {}", name);
        DuctResult::Success
    }

    fn dispatch_compute(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        println!("[dx12duct] ID3D12GraphicsCommandList::Dispatch({}, {}, {})", groups_x, groups_y, groups_z);
        DuctResult::Success
    }

    fn draw(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls += 1;
        println!(
            "[dx12duct] ID3D12GraphicsCommandList::DrawInstanced(VertexCount={}, StartVertex={})",
            vertex_count, first_vertex
        );
        DuctResult::Success
    }

    fn draw_indexed(&mut self, index_count: u32, first_index: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls += 1;
        println!(
            "[dx12duct] ID3D12GraphicsCommandList::DrawIndexedInstanced(IndexCount={}, StartIndex={})",
            index_count, first_index
        );
        DuctResult::Success
    }

    fn draw_instanced(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls += 1;
        println!(
            "[dx12duct] ID3D12GraphicsCommandList::DrawInstanced(Vertices={}, Instances={}, FirstVertex={}, FirstInstance={})",
            vertex_count, instance_count, first_vertex, first_instance
        );
        DuctResult::Success
    }

    fn draw_indexed_instanced(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        index_format: DuctIndexFormat,
    ) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls += 1;
        println!(
            "[dx12duct] ID3D12GraphicsCommandList::DrawIndexedInstanced(Indices={}, Instances={}, FirstIndex={}, Format={:?})",
            index_count, instance_count, first_index, index_format
        );
        DuctResult::Success
    }

    fn get_frame_stats(&mut self) -> Result<DuctFrameStats, DuctResult> {
        Ok(DuctFrameStats {
            fps: 60.0,
            frame_time_ms: 16.6,
            frame_count: self.frame_count,
            draw_calls: self.draw_calls,
            triangles_rendered: (self.draw_calls as u64) * 100,
            pipeline_switches: 1,
            allocated_gpu_memory_bytes: 1024 * 1024 * 8,
        })
    }

    fn get_raw_context(&mut self) -> Result<DuctRawContext, DuctResult> {
        Ok(DuctRawContext {
            backend_type: DuctBackendType::DirectX12,
            context_handle: self.device_mock_ptr,
            device_handle: self.command_queue_mock_ptr,
            extra_info: self.extra_info.as_ptr(),
            get_proc_address: Some(dx12_get_proc_address),
        })
    }

    fn execute_custom(&mut self, command_id: u32, data: *mut c_void) -> DuctResult {
        println!("[dx12duct] Custom D3D12 command invoked [id={}, payload={:?}]", command_id, data);
        DuctResult::Success
    }
}

export_pipeline!(Dx12Pipeline);
