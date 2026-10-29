use duct_core::*;
use std::ffi::c_void;

pub mod types;
pub mod device;
pub mod buffer;
pub mod pipeline;
pub mod texture;
pub mod frame;

pub use types::Dx11Pipeline;

impl RenderPipeline for Dx11Pipeline {
    fn backend_type(&self) -> DuctBackendType {
        DuctBackendType::DirectX11
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn init(&mut self, config: &DuctInitConfig) -> DuctResult {
        self.init_impl(config)
    }

    fn shutdown(&mut self) -> DuctResult {
        self.shutdown_impl()
    }

    fn resize_viewport(&mut self, width: u32, height: u32) -> DuctResult {
        self.resize_viewport_impl(width, height)
    }

    fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
        self.set_scissor_impl(x, y, width, height)
    }

    fn begin_frame(&mut self, clear_color: DuctColor) -> DuctResult {
        self.begin_frame_impl(clear_color)
    }

    fn end_frame(&mut self) -> DuctResult {
        self.end_frame_impl()
    }

    fn present(&mut self) -> DuctResult {
        self.present_impl()
    }

    fn create_shader(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        self.create_shader_impl(desc)
    }

    fn reload_shader(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        self.reload_shader_impl(handle, desc)
    }

    fn destroy_shader(&mut self, handle: DuctShaderHandle) -> DuctResult {
        self.destroy_shader_impl(handle)
    }

    fn create_buffer(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
        self.create_buffer_impl(desc)
    }

    fn update_buffer(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
        self.update_buffer_impl(handle, offset, data)
    }

    fn destroy_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        self.destroy_buffer_impl(handle)
    }

    fn bind_vertex_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        self.bind_vertex_buffer_impl(handle)
    }

    fn bind_index_buffer(&mut self, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult {
        self.bind_index_buffer_impl(handle, format)
    }

    fn bind_uniform_buffer(&mut self, slot: u32, handle: DuctBufferHandle) -> DuctResult {
        self.bind_uniform_buffer_impl(slot, handle)
    }

    fn create_texture_2d(
        &mut self,
        width: u32,
        height: u32,
        format: DuctTextureFormat,
        data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctResult> {
        self.create_texture_2d_impl(width, height, format, data)
    }

    fn update_texture_2d(
        &mut self,
        handle: DuctTextureHandle,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> DuctResult {
        self.update_texture_2d_impl(handle, x, y, width, height, data)
    }

    fn destroy_texture(&mut self, handle: DuctTextureHandle) -> DuctResult {
        self.destroy_texture_impl(handle)
    }

    fn bind_texture(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult {
        self.bind_texture_impl(slot, handle)
    }

    fn generate_mipmaps(&mut self, handle: DuctTextureHandle) -> DuctResult {
        self.generate_mipmaps_impl(handle)
    }

    fn set_texture_filter(
        &mut self,
        handle: DuctTextureHandle,
        min: DuctFilterMode,
        mag: DuctFilterMode,
    ) -> DuctResult {
        self.set_texture_filter_impl(handle, min, mag)
    }

    fn set_texture_wrap(
        &mut self,
        handle: DuctTextureHandle,
        wrap_s: DuctWrapMode,
        wrap_t: DuctWrapMode,
    ) -> DuctResult {
        self.set_texture_wrap_impl(handle, wrap_s, wrap_t)
    }

    fn create_render_target(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        self.create_render_target_impl(desc)
    }

    fn destroy_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        self.destroy_render_target_impl(handle)
    }

    fn bind_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        self.bind_render_target_impl(handle)
    }

    fn get_render_target_texture(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        self.get_render_target_texture_impl(handle)
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
        self.blit_render_target_impl(src, dst, src_w, src_h, dst_w, dst_h, filter)
    }

    fn read_pixels(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        out_rgba: &mut [u8],
    ) -> DuctResult {
        self.read_pixels_impl(x, y, width, height, out_rgba)
    }

    fn set_pipeline_state(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult {
        self.set_pipeline_state_impl(desc)
    }

    fn set_wireframe(&mut self, enabled: bool) -> DuctResult {
        self.set_wireframe_impl(enabled)
    }

    fn set_vsync(&mut self, enabled: bool) -> DuctResult {
        self.set_vsync_impl(enabled)
    }

    fn set_uniform_int(&mut self, name: &str, val: i32) -> DuctResult {
        self.set_uniform_int_impl(name, val)
    }

    fn set_uniform_float(&mut self, name: &str, val: f32) -> DuctResult {
        self.set_uniform_float_impl(name, val)
    }

    fn set_uniform_vec2(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        self.set_uniform_vec2_impl(name, x, y)
    }

    fn set_uniform_vec3(&mut self, name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        self.set_uniform_vec3_impl(name, x, y, z)
    }

    fn set_uniform_vec4(&mut self, name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        self.set_uniform_vec4_impl(name, x, y, z, w)
    }

    fn set_uniform_mat4(&mut self, name: &str, elements: &[f32; 16]) -> DuctResult {
        self.set_uniform_mat4_impl(name, elements)
    }

    fn draw(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        self.draw_impl(vertex_count, first_vertex)
    }

    fn draw_indexed(&mut self, index_count: u32, first_index: u32) -> DuctResult {
        self.draw_indexed_impl(index_count, first_index)
    }

    fn draw_instanced(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> DuctResult {
        self.draw_instanced_impl(vertex_count, instance_count, first_vertex, first_instance)
    }

    fn draw_indexed_instanced(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        index_format: DuctIndexFormat,
    ) -> DuctResult {
        self.draw_indexed_instanced_impl(index_count, instance_count, first_index, index_format)
    }

    fn dispatch_compute(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        self.dispatch_compute_impl(groups_x, groups_y, groups_z)
    }

    fn get_frame_stats(&mut self) -> Result<DuctFrameStats, DuctResult> {
        self.get_frame_stats_impl()
    }

    fn get_raw_context(&mut self) -> Result<DuctRawContext, DuctResult> {
        self.get_raw_context_impl()
    }

    fn execute_custom(&mut self, command_id: u32, data: *mut c_void) -> DuctResult {
        self.execute_custom_impl(command_id, data)
    }
}

export_pipeline!(Dx11Pipeline, dx11duct_create_pipeline);
