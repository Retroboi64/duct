use crate::vk;
use crate::vk::*;
use duct_core::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    pub(crate) fn IsWindow(hWnd: *mut c_void) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    pub(crate) fn GetModuleHandleW(lpModuleName: *const u16) -> *mut c_void;
}

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct VkPushConstantBlock {
    pub(crate) color: [f32; 4],      // offset 0:  uColor (default [1.0, 1.0, 1.0, 1.0])
    pub(crate) params: [f32; 4],     // offset 16: uParams ([off_x, off_y, scale_x, scale_y])
    pub(crate) transform: [f32; 16], // offset 32: uTransform (4x4 column-major matrix)
    pub(crate) extra: [f32; 4],      // offset 96: uExtra ([time, res_x, res_y, mode])
}

impl Default for VkPushConstantBlock {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            params: [0.0, 0.0, 1.0, 1.0],
            transform: [
                1.0, 0.0, 0.0, 0.0, // col 0
                0.0, 1.0, 0.0, 0.0, // col 1
                0.0, 0.0, 1.0, 0.0, // col 2
                0.0, 0.0, 0.0, 1.0, // col 3
            ],
            extra: [0.0, 1280.0, 720.0, 1.0],
        }
    }
}

pub(crate) struct VkShaderRecord {
    pub(crate) module: VkShaderModule,
    pub(crate) module_with_color: VkShaderModule,
    pub(crate) stage: DuctShaderStage,
    pub(crate) is_custom_spv: bool,
}

pub(crate) struct VkBufferRecord {
    pub(crate) buffer: VkBuffer,
    pub(crate) memory: VkDeviceMemory,
    pub(crate) size_bytes: usize,
    pub(crate) _buffer_type: DuctBufferType,
}

#[derive(Copy, Clone)]
pub(crate) struct VkTextureRecord {
    pub(crate) image: VkImage,
    pub(crate) memory: VkDeviceMemory,
    pub(crate) view: VkImageView,
    pub(crate) sampler: VkSampler,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) vk_format: u32,
    pub(crate) min_filter: DuctFilterMode,
    pub(crate) mag_filter: DuctFilterMode,
    pub(crate) wrap_s: DuctWrapMode,
    pub(crate) wrap_t: DuctWrapMode,
}

pub(crate) struct VkRenderTargetRecord {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) color_tex: DuctTextureHandle,
    pub(crate) depth_tex: Option<DuctTextureHandle>,
    pub(crate) depth_image: VkImage,
    pub(crate) depth_memory: VkDeviceMemory,
    pub(crate) depth_view: VkImageView,
    pub(crate) framebuffer: VkFramebuffer,
    pub(crate) has_depth: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct VkPipelineKey {
    pub(crate) vs: u64,
    pub(crate) fs: u64,
    pub(crate) cull_mode: u32,
    pub(crate) blend_mode: u32,
    pub(crate) depth_test: bool,
    pub(crate) depth_write: bool,
    pub(crate) depth_compare: u32,
    pub(crate) wireframe: bool,
    pub(crate) is_offscreen: bool,
    pub(crate) stride: u32,
    pub(crate) attr_count: u32,
    pub(crate) attr_formats: [u32; 8],
    pub(crate) attr_offsets: [u32; 8],
}

pub struct VkPipeline {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) extra_info: CString,
    pub(crate) initialized: bool,

    // Real Vulkan runtime state
    pub(crate) loader: Option<VulkanLoader>,
    pub(crate) instance: VkInstance,
    pub(crate) instance_fns: Option<VkInstanceFns>,
    pub(crate) physical_device: VkPhysicalDevice,
    pub(crate) device: VkDevice,
    pub(crate) device_fns: Option<VkDeviceFns>,
    pub(crate) queue: VkQueue,
    pub(crate) graphics_queue_family: u32,
    pub(crate) memory_properties: VkPhysicalDeviceMemoryProperties,

    // Swapchain & Presentation
    pub(crate) surface: VkSurfaceKHR,
    pub(crate) swapchain: VkSwapchainKHR,
    pub(crate) swapchain_format: u32,
    pub(crate) swapchain_extent: VkExtent2D,
    pub(crate) swapchain_images: Vec<VkImage>,
    pub(crate) swapchain_image_views: Vec<VkImageView>,
    pub(crate) render_pass: VkRenderPass,
    pub(crate) offscreen_render_pass: VkRenderPass,
    pub(crate) offscreen_render_pass_with_depth: VkRenderPass,
    pub(crate) depth_image: VkImage,
    pub(crate) depth_image_memory: VkDeviceMemory,
    pub(crate) depth_image_view: VkImageView,
    pub(crate) framebuffers: Vec<VkFramebuffer>,
    pub(crate) current_image_index: u32,
    pub(crate) swapchain_active: bool,
    pub(crate) swapchain_image_acquired: bool,
    pub(crate) render_pass_active: bool,
    pub(crate) current_pass_is_offscreen: bool,
    pub(crate) vsync_enabled: bool,

    // Pipelines, Shaders & Push Constants
    pub(crate) pipeline_layout: VkPipelineLayout,
    pub(crate) compute_pipeline_layout: VkPipelineLayout,
    pub(crate) default_vs_module: VkShaderModule,
    pub(crate) default_vs_color_module: VkShaderModule,
    pub(crate) default_fs_module: VkShaderModule,
    pub(crate) graphics_pipelines: HashMap<VkPipelineKey, vk::VkPipeline>,
    pub(crate) compute_pipelines: HashMap<DuctShaderHandle, vk::VkPipeline>,
    pub(crate) current_pso: Option<DuctPipelineStateDesc>,
    pub(crate) active_graphics_pipeline: vk::VkPipeline,
    pub(crate) active_compute_pipeline: vk::VkPipeline,
    pub(crate) wireframe: bool,
    pub(crate) push_constants: VkPushConstantBlock,

    // Descriptors & Textures
    pub(crate) descriptor_set_layout: VkDescriptorSetLayout,
    pub(crate) descriptor_pool: VkDescriptorPool,
    pub(crate) descriptor_set: VkDescriptorSet,
    pub(crate) bound_textures: [DuctTextureHandle; 8],
    pub(crate) descriptor_set_dirty: bool,
    pub(crate) default_white_tex: DuctTextureHandle,

    // Command Execution & Sync
    pub(crate) command_pool: VkCommandPool,
    pub(crate) command_buffer: VkCommandBuffer,
    pub(crate) cmd_recording: bool,
    pub(crate) image_available_sem: VkSemaphore,
    pub(crate) render_finished_sem: VkSemaphore,
    pub(crate) in_flight_fence: VkFence,

    // Dimensions & Telemetry
    pub(crate) viewport_width: u32,
    pub(crate) viewport_height: u32,
    pub(crate) frame_in_progress: bool,
    pub(crate) frame_count: u64,
    pub(crate) draw_calls_in_frame: u32,
    pub(crate) vertices_drawn_in_frame: u32,
    pub(crate) pipeline_switches_in_frame: u32,
    pub(crate) allocated_gpu_bytes: usize,
    pub(crate) last_frame_instant: Option<std::time::Instant>,
    pub(crate) current_frame_time_ms: f32,
    pub(crate) smoothed_fps: f32,

    // Resource Tracking
    pub(crate) next_handle_id: u64,
    pub(crate) shaders: HashMap<DuctShaderHandle, VkShaderRecord>,
    pub(crate) buffers: HashMap<DuctBufferHandle, VkBufferRecord>,
    pub(crate) textures: HashMap<DuctTextureHandle, VkTextureRecord>,
    pub(crate) render_targets: HashMap<DuctRenderTargetHandle, VkRenderTargetRecord>,
    pub(crate) active_render_target: DuctRenderTargetHandle,
    pub(crate) last_bound_vbo: VkBuffer,
    pub(crate) last_bound_ibo: VkBuffer,
    pub(crate) current_index_format: DuctIndexFormat,
    pub(crate) last_clear_color: DuctColor,
}

impl Default for VkPipeline {
    fn default() -> Self {
        Self {
            name: "Vulkan Pipeline (API 1.3)".to_string(),
            version: "1.3.0".to_string(),
            extra_info: CString::new("Vulkan 1.3 - Project32 Duct Hardware Vulkan Engine").unwrap(),
            initialized: false,
            loader: None,
            instance: std::ptr::null_mut(),
            instance_fns: None,
            physical_device: std::ptr::null_mut(),
            device: std::ptr::null_mut(),
            device_fns: None,
            queue: std::ptr::null_mut(),
            graphics_queue_family: 0,
            memory_properties: VkPhysicalDeviceMemoryProperties::default(),
            surface: VK_NULL_HANDLE,
            swapchain: VK_NULL_HANDLE,
            swapchain_format: VK_FORMAT_B8G8R8A8_UNORM,
            swapchain_extent: VkExtent2D { width: 800, height: 600 },
            swapchain_images: Vec::new(),
            swapchain_image_views: Vec::new(),
            render_pass: VK_NULL_HANDLE,
            offscreen_render_pass: VK_NULL_HANDLE,
            offscreen_render_pass_with_depth: VK_NULL_HANDLE,
            depth_image: VK_NULL_HANDLE,
            depth_image_memory: VK_NULL_HANDLE,
            depth_image_view: VK_NULL_HANDLE,
            framebuffers: Vec::new(),
            current_image_index: 0,
            swapchain_active: false,
            swapchain_image_acquired: false,
            render_pass_active: false,
            current_pass_is_offscreen: false,
            vsync_enabled: true,
            pipeline_layout: VK_NULL_HANDLE,
            compute_pipeline_layout: VK_NULL_HANDLE,
            default_vs_module: VK_NULL_HANDLE,
            default_vs_color_module: VK_NULL_HANDLE,
            default_fs_module: VK_NULL_HANDLE,
            graphics_pipelines: HashMap::new(),
            compute_pipelines: HashMap::new(),
            current_pso: None,
            active_graphics_pipeline: VK_NULL_HANDLE,
            active_compute_pipeline: VK_NULL_HANDLE,
            wireframe: false,
            push_constants: VkPushConstantBlock::default(),
            descriptor_set_layout: VK_NULL_HANDLE,
            descriptor_pool: VK_NULL_HANDLE,
            descriptor_set: VK_NULL_HANDLE,
            bound_textures: [DuctTextureHandle::INVALID; 8],
            descriptor_set_dirty: false,
            default_white_tex: DuctTextureHandle::INVALID,
            command_pool: VK_NULL_HANDLE,
            command_buffer: std::ptr::null_mut(),
            cmd_recording: false,
            image_available_sem: VK_NULL_HANDLE,
            render_finished_sem: VK_NULL_HANDLE,
            in_flight_fence: VK_NULL_HANDLE,
            viewport_width: 800,
            viewport_height: 600,
            frame_in_progress: false,
            frame_count: 0,
            draw_calls_in_frame: 0,
            vertices_drawn_in_frame: 0,
            pipeline_switches_in_frame: 0,
            allocated_gpu_bytes: 0,
            last_frame_instant: None,
            current_frame_time_ms: 16.6,
            smoothed_fps: 60.0,
            next_handle_id: 1,
            shaders: HashMap::new(),
            buffers: HashMap::new(),
            textures: HashMap::new(),
            render_targets: HashMap::new(),
            active_render_target: DuctRenderTargetHandle::INVALID,
            last_bound_vbo: VK_NULL_HANDLE,
            last_bound_ibo: VK_NULL_HANDLE,
            current_index_format: DuctIndexFormat::U32,
            last_clear_color: DuctColor::BLACK,
        }
    }
}


pub(crate) unsafe extern "C" fn vk_get_proc_address(_name: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}
pub(crate) fn map_vertex_format(fmt: DuctVertexFormat) -> u32 {
    match fmt {
        DuctVertexFormat::Float => VK_FORMAT_R32_SFLOAT,
        DuctVertexFormat::Float2 => VK_FORMAT_R32G32_SFLOAT,
        DuctVertexFormat::Float3 => VK_FORMAT_R32G32B32_SFLOAT,
        DuctVertexFormat::Float4 => VK_FORMAT_R32G32B32A32_SFLOAT,
        DuctVertexFormat::UByte4Normalized => VK_FORMAT_R8G8B8A8_UNORM,
    }
}

pub(crate) fn map_texture_format(fmt: DuctTextureFormat) -> u32 {
    match fmt {
        DuctTextureFormat::Rgba8 => VK_FORMAT_R8G8B8A8_UNORM,
        DuctTextureFormat::Rgb8 => VK_FORMAT_R8G8B8A8_UNORM,
        DuctTextureFormat::R8 => VK_FORMAT_R8_UNORM,
        DuctTextureFormat::Rgba16Float => VK_FORMAT_R16G16B16A16_SFLOAT,
        DuctTextureFormat::Depth32Float => VK_FORMAT_D32_SFLOAT,
    }
}

pub(crate) fn map_filter(mode: DuctFilterMode) -> u32 {
    match mode {
        DuctFilterMode::Nearest => VK_FILTER_NEAREST,
        DuctFilterMode::Linear | DuctFilterMode::LinearMipmapLinear => VK_FILTER_LINEAR,
    }
}

pub(crate) fn map_address_mode(wrap: DuctWrapMode) -> u32 {
    match wrap {
        DuctWrapMode::Repeat => VK_SAMPLER_ADDRESS_MODE_REPEAT,
        DuctWrapMode::ClampToEdge => VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
        DuctWrapMode::MirroredRepeat => VK_SAMPLER_ADDRESS_MODE_MIRRORED_REPEAT,
    }
}

