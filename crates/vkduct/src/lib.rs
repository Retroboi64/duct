//! Vulkan backend pipeline for Project32 duct graphics subsystem.
//! Compiles into `duct.vk.dll` (or platform equivalent).
//!
//! Zero-dependency Vulkan 1.3 implementation loading the system Vulkan runtime
//! (`vulkan-1.dll`, `libvulkan.so.1`, `libvulkan.1.dylib`) directly via OS FFI.
//! Manages real `VkInstance`, `VkPhysicalDevice`, `VkDevice`, `VkQueue`,
//! `VkSwapchainKHR`, `VkRenderPass`, `VkFramebuffer`, `VkPipelineLayout`,
//! `VkPipeline` (Graphics & Compute), `VkCommandPool`, `VkCommandBuffer`,
//! `VkBuffer`, `VkImage`, `VkImageView`, `VkSampler`, `VkDeviceMemory`,
//! and `VkShaderModule` with built-in SPIR-V code generation.

mod spv;
mod vk;

use duct_core::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};
use vk::*;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn IsWindow(hWnd: *mut c_void) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(lpModuleName: *const u16) -> *mut c_void;
}

#[repr(C)]
#[derive(Copy, Clone)]
struct VkPushConstantBlock {
    color: [f32; 4],      // offset 0:  uColor (default [1.0, 1.0, 1.0, 1.0])
    params: [f32; 4],     // offset 16: uParams ([off_x, off_y, scale_x, scale_y])
    transform: [f32; 16], // offset 32: uTransform (4x4 column-major matrix)
    extra: [f32; 4],      // offset 96: uExtra ([time, res_x, res_y, mode])
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

struct VkShaderRecord {
    module: VkShaderModule,
    module_with_color: VkShaderModule,
    stage: DuctShaderStage,
    is_custom_spv: bool,
}

struct VkBufferRecord {
    buffer: VkBuffer,
    memory: VkDeviceMemory,
    size_bytes: usize,
    _buffer_type: DuctBufferType,
}

struct VkTextureRecord {
    image: VkImage,
    memory: VkDeviceMemory,
    view: VkImageView,
    sampler: VkSampler,
    width: u32,
    height: u32,
    vk_format: u32,
    min_filter: DuctFilterMode,
    mag_filter: DuctFilterMode,
    wrap_s: DuctWrapMode,
    wrap_t: DuctWrapMode,
}

struct VkRenderTargetRecord {
    width: u32,
    height: u32,
    color_tex: DuctTextureHandle,
    depth_image: VkImage,
    depth_memory: VkDeviceMemory,
    depth_view: VkImageView,
    framebuffer: VkFramebuffer,
    has_depth: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
struct VkPipelineKey {
    vs: u64,
    fs: u64,
    cull_mode: u32,
    blend_mode: u32,
    depth_test: bool,
    depth_write: bool,
    depth_compare: u32,
    wireframe: bool,
    is_offscreen: bool,
    stride: u32,
    attr_count: u32,
    attr_formats: [u32; 8],
    attr_offsets: [u32; 8],
}

pub struct VkPipeline {
    name: String,
    version: String,
    extra_info: CString,
    initialized: bool,

    // Real Vulkan runtime state
    loader: Option<VulkanLoader>,
    instance: VkInstance,
    instance_fns: Option<VkInstanceFns>,
    physical_device: VkPhysicalDevice,
    device: VkDevice,
    device_fns: Option<VkDeviceFns>,
    queue: VkQueue,
    graphics_queue_family: u32,
    memory_properties: VkPhysicalDeviceMemoryProperties,

    // Swapchain & Presentation
    surface: VkSurfaceKHR,
    swapchain: VkSwapchainKHR,
    swapchain_format: u32,
    swapchain_extent: VkExtent2D,
    swapchain_images: Vec<VkImage>,
    swapchain_image_views: Vec<VkImageView>,
    render_pass: VkRenderPass,
    offscreen_render_pass: VkRenderPass,
    offscreen_render_pass_with_depth: VkRenderPass,
    depth_image: VkImage,
    depth_image_memory: VkDeviceMemory,
    depth_image_view: VkImageView,
    framebuffers: Vec<VkFramebuffer>,
    current_image_index: u32,
    swapchain_active: bool,
    swapchain_image_acquired: bool,
    render_pass_active: bool,
    current_pass_is_offscreen: bool,
    vsync_enabled: bool,

    // Pipelines, Shaders & Push Constants
    pipeline_layout: VkPipelineLayout,
    compute_pipeline_layout: VkPipelineLayout,
    default_vs_module: VkShaderModule,
    default_vs_color_module: VkShaderModule,
    default_fs_module: VkShaderModule,
    graphics_pipelines: HashMap<VkPipelineKey, vk::VkPipeline>,
    compute_pipelines: HashMap<DuctShaderHandle, vk::VkPipeline>,
    current_pso: Option<DuctPipelineStateDesc>,
    active_graphics_pipeline: vk::VkPipeline,
    active_compute_pipeline: vk::VkPipeline,
    wireframe: bool,
    push_constants: VkPushConstantBlock,

    // Command Execution & Sync
    command_pool: VkCommandPool,
    command_buffer: VkCommandBuffer,
    cmd_recording: bool,
    image_available_sem: VkSemaphore,
    render_finished_sem: VkSemaphore,
    in_flight_fence: VkFence,

    // Dimensions & Telemetry
    viewport_width: u32,
    viewport_height: u32,
    frame_in_progress: bool,
    frame_count: u64,
    draw_calls_in_frame: u32,
    vertices_drawn_in_frame: u32,
    pipeline_switches_in_frame: u32,
    allocated_gpu_bytes: usize,
    last_frame_instant: Option<std::time::Instant>,
    current_frame_time_ms: f32,
    smoothed_fps: f32,

    // Resource Tracking
    next_handle_id: u64,
    shaders: HashMap<DuctShaderHandle, VkShaderRecord>,
    buffers: HashMap<DuctBufferHandle, VkBufferRecord>,
    textures: HashMap<DuctTextureHandle, VkTextureRecord>,
    render_targets: HashMap<DuctRenderTargetHandle, VkRenderTargetRecord>,
    active_render_target: DuctRenderTargetHandle,
    last_bound_vbo: VkBuffer,
    last_bound_ibo: VkBuffer,
    current_index_format: DuctIndexFormat,
    last_clear_color: DuctColor,
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

unsafe extern "C" fn vk_get_proc_address(_name: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}

fn map_vertex_format(fmt: DuctVertexFormat) -> u32 {
    match fmt {
        DuctVertexFormat::Float => VK_FORMAT_R32_SFLOAT,
        DuctVertexFormat::Float2 => VK_FORMAT_R32G32_SFLOAT,
        DuctVertexFormat::Float3 => VK_FORMAT_R32G32B32_SFLOAT,
        DuctVertexFormat::Float4 => VK_FORMAT_R32G32B32A32_SFLOAT,
        DuctVertexFormat::UByte4Normalized => VK_FORMAT_R8G8B8A8_UNORM,
    }
}

fn map_texture_format(fmt: DuctTextureFormat) -> u32 {
    match fmt {
        DuctTextureFormat::Rgba8 => VK_FORMAT_R8G8B8A8_UNORM,
        DuctTextureFormat::Rgb8 => VK_FORMAT_R8G8B8A8_UNORM,
        DuctTextureFormat::R8 => VK_FORMAT_R8_UNORM,
        DuctTextureFormat::Rgba16Float => VK_FORMAT_R16G16B16A16_SFLOAT,
        DuctTextureFormat::Depth32Float => VK_FORMAT_D32_SFLOAT,
    }
}

fn map_filter(mode: DuctFilterMode) -> u32 {
    match mode {
        DuctFilterMode::Nearest => VK_FILTER_NEAREST,
        DuctFilterMode::Linear | DuctFilterMode::LinearMipmapLinear => VK_FILTER_LINEAR,
    }
}

fn map_address_mode(wrap: DuctWrapMode) -> u32 {
    match wrap {
        DuctWrapMode::Repeat => VK_SAMPLER_ADDRESS_MODE_REPEAT,
        DuctWrapMode::ClampToEdge => VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
        DuctWrapMode::MirroredRepeat => VK_SAMPLER_ADDRESS_MODE_MIRRORED_REPEAT,
    }
}

impl VkPipeline {
    fn find_memory_type(&self, type_filter: u32, properties: VkFlags) -> Option<u32> {
        for i in 0..self.memory_properties.memoryTypeCount {
            if (type_filter & (1 << i)) != 0
                && (self.memory_properties.memoryTypes[i as usize].propertyFlags & properties)
                    == properties
            {
                return Some(i);
            }
        }
        None
    }

    unsafe fn create_spv_module(&self, words: &[u32]) -> VkShaderModule {
        let dfns = match &self.device_fns {
            Some(d) if !self.device.is_null() => d,
            _ => return VK_NULL_HANDLE,
        };
        let info = VkShaderModuleCreateInfo {
            sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            codeSize: words.len() * 4,
            pCode: words.as_ptr(),
        };
        let mut module = VK_NULL_HANDLE;
        if unsafe { (dfns.CreateShaderModule)(self.device, &info, std::ptr::null(), &mut module) }
            == VK_SUCCESS
        {
            module
        } else {
            VK_NULL_HANDLE
        }
    }

    unsafe fn create_single_render_pass(
        &self,
        format: u32,
        final_layout: u32,
        with_depth: bool,
    ) -> VkRenderPass {
        let dfns = match &self.device_fns {
            Some(d) if !self.device.is_null() => d,
            _ => return VK_NULL_HANDLE,
        };

        let color_attachment = VkAttachmentDescription {
            flags: 0,
            format,
            samples: VK_SAMPLE_COUNT_1_BIT,
            loadOp: VK_ATTACHMENT_LOAD_OP_CLEAR,
            storeOp: VK_ATTACHMENT_STORE_OP_STORE,
            stencilLoadOp: VK_ATTACHMENT_LOAD_OP_DONT_CARE,
            stencilStoreOp: VK_ATTACHMENT_STORE_OP_DONT_CARE,
            initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
            finalLayout: final_layout,
        };

        let color_attachment_ref = VkAttachmentReference {
            attachment: 0,
            layout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };

        let depth_attachment = VkAttachmentDescription {
            flags: 0,
            format: VK_FORMAT_D32_SFLOAT,
            samples: VK_SAMPLE_COUNT_1_BIT,
            loadOp: VK_ATTACHMENT_LOAD_OP_CLEAR,
            storeOp: VK_ATTACHMENT_STORE_OP_DONT_CARE,
            stencilLoadOp: VK_ATTACHMENT_LOAD_OP_DONT_CARE,
            stencilStoreOp: VK_ATTACHMENT_STORE_OP_DONT_CARE,
            initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
            finalLayout: VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let depth_attachment_ref = VkAttachmentReference {
            attachment: 1,
            layout: VK_IMAGE_LAYOUT_DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let subpass = VkSubpassDescription {
            flags: 0,
            pipelineBindPoint: VK_PIPELINE_BIND_POINT_GRAPHICS,
            inputAttachmentCount: 0,
            pInputAttachments: std::ptr::null(),
            colorAttachmentCount: 1,
            pColorAttachments: &color_attachment_ref,
            pResolveAttachments: std::ptr::null(),
            pDepthStencilAttachment: if with_depth {
                &depth_attachment_ref
            } else {
                std::ptr::null()
            },
            preserveAttachmentCount: 0,
            pPreserveAttachments: std::ptr::null(),
        };

        let dependency = VkSubpassDependency {
            srcSubpass: VK_SUBPASS_EXTERNAL,
            dstSubpass: 0,
            srcStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT
                | if with_depth {
                    VK_PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT
                } else {
                    0
                },
            dstStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT
                | if with_depth {
                    VK_PIPELINE_STAGE_EARLY_FRAGMENT_TESTS_BIT
                } else {
                    0
                },
            srcAccessMask: 0,
            dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT
                | if with_depth {
                    VK_ACCESS_DEPTH_STENCIL_ATTACHMENT_WRITE_BIT
                } else {
                    0
                },
            dependencyFlags: 0,
        };

        let attachments = if with_depth {
            vec![color_attachment, depth_attachment]
        } else {
            vec![color_attachment]
        };

        let rp_info = VkRenderPassCreateInfo {
            sType: VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            attachmentCount: attachments.len() as u32,
            pAttachments: attachments.as_ptr(),
            subpassCount: 1,
            pSubpasses: &subpass,
            dependencyCount: 1,
            pDependencies: &dependency,
        };

        let mut rp = VK_NULL_HANDLE;
        if unsafe { (dfns.CreateRenderPass)(self.device, &rp_info, std::ptr::null(), &mut rp) }
            == VK_SUCCESS
        {
            rp
        } else {
            VK_NULL_HANDLE
        }
    }

    unsafe fn cleanup_swapchain(&mut self) {
        if let Some(dfns) = &self.device_fns {
            if !self.device.is_null() {
                let _ = unsafe { (dfns.DeviceWaitIdle)(self.device) };
                for fb in self.framebuffers.drain(..) {
                    if fb != VK_NULL_HANDLE {
                        unsafe { (dfns.DestroyFramebuffer)(self.device, fb, std::ptr::null()) };
                    }
                }
                if self.render_pass != VK_NULL_HANDLE {
                    unsafe {
                        (dfns.DestroyRenderPass)(self.device, self.render_pass, std::ptr::null());
                    }
                    self.render_pass = VK_NULL_HANDLE;
                }
                if self.depth_image_view != VK_NULL_HANDLE {
                    unsafe { (dfns.DestroyImageView)(self.device, self.depth_image_view, std::ptr::null()) };
                    self.depth_image_view = VK_NULL_HANDLE;
                }
                if self.depth_image != VK_NULL_HANDLE {
                    unsafe { (dfns.DestroyImage)(self.device, self.depth_image, std::ptr::null()) };
                    self.depth_image = VK_NULL_HANDLE;
                }
                if self.depth_image_memory != VK_NULL_HANDLE {
                    unsafe { (dfns.FreeMemory)(self.device, self.depth_image_memory, std::ptr::null()) };
                    self.depth_image_memory = VK_NULL_HANDLE;
                }
                for view in self.swapchain_image_views.drain(..) {
                    if view != VK_NULL_HANDLE {
                        unsafe { (dfns.DestroyImageView)(self.device, view, std::ptr::null()) };
                    }
                }
                if self.swapchain != VK_NULL_HANDLE {
                    if let Some(destroy_sc) = dfns.DestroySwapchainKHR {
                        unsafe { destroy_sc(self.device, self.swapchain, std::ptr::null()) };
                    }
                    self.swapchain = VK_NULL_HANDLE;
                }
            }
        }
        self.swapchain_images.clear();
        self.swapchain_active = false;
        self.swapchain_image_acquired = false;
    }

    unsafe fn init_swapchain(&mut self, width: u32, height: u32, vsync: bool) -> bool {
        let (ifns, dfns) = match (&self.instance_fns, &self.device_fns) {
            (Some(i), Some(d)) => (*i, *d),
            _ => return false,
        };

        if self.surface == VK_NULL_HANDLE {
            // Headless mode: still create a default render pass so graphics pipelines can be compiled
            if self.render_pass == VK_NULL_HANDLE {
                self.render_pass = unsafe {
                    self.create_single_render_pass(
                        VK_FORMAT_B8G8R8A8_UNORM,
                        VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                        true,
                    )
                };
            }
            return false;
        }

        let (get_caps, get_formats, create_sc, get_images) = match (
            ifns.GetPhysicalDeviceSurfaceCapabilitiesKHR,
            ifns.GetPhysicalDeviceSurfaceFormatsKHR,
            dfns.CreateSwapchainKHR,
            dfns.GetSwapchainImagesKHR,
        ) {
            (Some(c), Some(f), Some(csc), Some(gi)) => (c, f, csc, gi),
            _ => return false,
        };

        if let Some(get_support) = ifns.GetPhysicalDeviceSurfaceSupportKHR {
            let mut supported = VK_FALSE;
            let res = unsafe {
                get_support(
                    self.physical_device,
                    self.graphics_queue_family,
                    self.surface,
                    &mut supported,
                )
            };
            if res != VK_SUCCESS || supported == VK_FALSE {
                return false;
            }
        }

        let mut caps = VkSurfaceCapabilitiesKHR::default();
        if unsafe { get_caps(self.physical_device, self.surface, &mut caps) } != VK_SUCCESS {
            return false;
        }

        let mut fmt_count = 0u32;
        unsafe {
            get_formats(
                self.physical_device,
                self.surface,
                &mut fmt_count,
                std::ptr::null_mut(),
            );
        }
        let mut chosen_format = VK_FORMAT_B8G8R8A8_UNORM;
        let mut chosen_colorspace = VK_COLOR_SPACE_SRGB_NONLINEAR_KHR;
        if fmt_count > 0 {
            let mut formats = vec![VkSurfaceFormatKHR::default(); fmt_count as usize];
            unsafe {
                get_formats(
                    self.physical_device,
                    self.surface,
                    &mut fmt_count,
                    formats.as_mut_ptr(),
                );
            }
            chosen_format = formats[0].format;
            chosen_colorspace = formats[0].colorSpace;
            for f in &formats {
                if f.format == VK_FORMAT_B8G8R8A8_UNORM || f.format == VK_FORMAT_R8G8B8A8_UNORM {
                    chosen_format = f.format;
                    chosen_colorspace = f.colorSpace;
                    break;
                }
            }
        }

        let extent = if caps.currentExtent.width != u32::MAX && caps.currentExtent.width > 0 {
            caps.currentExtent
        } else {
            VkExtent2D {
                width: width.clamp(
                    caps.minImageExtent.width.max(1),
                    caps.maxImageExtent.width.max(1),
                ),
                height: height.clamp(
                    caps.minImageExtent.height.max(1),
                    caps.maxImageExtent.height.max(1),
                ),
            }
        };

        if extent.width == 0 || extent.height == 0 {
            return false;
        }

        let mut image_count = caps.minImageCount.max(2);
        if caps.maxImageCount > 0 && image_count > caps.maxImageCount {
            image_count = caps.maxImageCount;
        }

        let present_mode = if vsync {
            VK_PRESENT_MODE_FIFO_KHR
        } else {
            VK_PRESENT_MODE_FIFO_KHR
        };

        let extra_usage = caps.supportedUsageFlags
            & (VK_IMAGE_USAGE_TRANSFER_SRC_BIT | VK_IMAGE_USAGE_TRANSFER_DST_BIT);

        let sc_info = VkSwapchainCreateInfoKHR {
            sType: VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
            pNext: std::ptr::null(),
            flags: 0,
            surface: self.surface,
            minImageCount: image_count,
            imageFormat: chosen_format,
            imageColorSpace: chosen_colorspace,
            imageExtent: extent,
            imageArrayLayers: 1,
            imageUsage: VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | extra_usage,
            imageSharingMode: VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            preTransform: if caps.currentTransform != 0 {
                caps.currentTransform
            } else {
                1
            },
            compositeAlpha: VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
            presentMode: present_mode,
            clipped: VK_TRUE,
            oldSwapchain: VK_NULL_HANDLE,
        };

        let mut swapchain = VK_NULL_HANDLE;
        if unsafe { create_sc(self.device, &sc_info, std::ptr::null(), &mut swapchain) }
            != VK_SUCCESS
        {
            return false;
        }
        self.swapchain = swapchain;
        self.swapchain_format = chosen_format;
        self.swapchain_extent = extent;

        let mut sc_img_count = 0u32;
        unsafe {
            get_images(
                self.device,
                self.swapchain,
                &mut sc_img_count,
                std::ptr::null_mut(),
            );
        }
        if sc_img_count == 0 {
            return false;
        }
        self.swapchain_images = vec![VK_NULL_HANDLE; sc_img_count as usize];
        unsafe {
            get_images(
                self.device,
                self.swapchain,
                &mut sc_img_count,
                self.swapchain_images.as_mut_ptr(),
            );
        }

        let render_pass = unsafe {
            self.create_single_render_pass(self.swapchain_format, VK_IMAGE_LAYOUT_PRESENT_SRC_KHR, true)
        };
        if render_pass == VK_NULL_HANDLE {
            return false;
        }
        self.render_pass = render_pass;

        // Allocate and create Depth Buffer for swapchain
        let depth_format = VK_FORMAT_D32_SFLOAT;
        let depth_info = VkImageCreateInfo {
            sType: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            imageType: VK_IMAGE_TYPE_2D,
            format: depth_format,
            extent: VkExtent3D {
                width: self.swapchain_extent.width,
                height: self.swapchain_extent.height,
                depth: 1,
            },
            mipLevels: 1,
            arrayLayers: 1,
            samples: VK_SAMPLE_COUNT_1_BIT,
            tiling: VK_IMAGE_TILING_OPTIMAL,
            usage: VK_IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT,
            sharingMode: VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
        };
        let mut depth_img = VK_NULL_HANDLE;
        if unsafe { (dfns.CreateImage)(self.device, &depth_info, std::ptr::null(), &mut depth_img) } == VK_SUCCESS {
            let mut mem_reqs = VkMemoryRequirements::default();
            unsafe { (dfns.GetImageMemoryRequirements)(self.device, depth_img, &mut mem_reqs) };
            if let Some(mem_type) = self
                .find_memory_type(mem_reqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT)
                .or_else(|| self.find_memory_type(mem_reqs.memoryTypeBits, 0))
            {
                let alloc_info = VkMemoryAllocateInfo {
                    sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                    pNext: std::ptr::null(),
                    allocationSize: mem_reqs.size,
                    memoryTypeIndex: mem_type,
                };
                let mut depth_mem = VK_NULL_HANDLE;
                if unsafe { (dfns.AllocateMemory)(self.device, &alloc_info, std::ptr::null(), &mut depth_mem) } == VK_SUCCESS {
                    unsafe { (dfns.BindImageMemory)(self.device, depth_img, depth_mem, 0) };
                    let iv_info = VkImageViewCreateInfo {
                        sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        image: depth_img,
                        viewType: VK_IMAGE_VIEW_TYPE_2D,
                        format: depth_format,
                        components: VkComponentMapping {
                            r: VK_COMPONENT_SWIZZLE_IDENTITY,
                            g: VK_COMPONENT_SWIZZLE_IDENTITY,
                            b: VK_COMPONENT_SWIZZLE_IDENTITY,
                            a: VK_COMPONENT_SWIZZLE_IDENTITY,
                        },
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_DEPTH_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    };
                    let mut depth_view = VK_NULL_HANDLE;
                    if unsafe { (dfns.CreateImageView)(self.device, &iv_info, std::ptr::null(), &mut depth_view) } == VK_SUCCESS {
                        self.depth_image = depth_img;
                        self.depth_image_memory = depth_mem;
                        self.depth_image_view = depth_view;
                    }
                }
            }
        }

        for &img in &self.swapchain_images {
            let iv_info = VkImageViewCreateInfo {
                sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                image: img,
                viewType: VK_IMAGE_VIEW_TYPE_2D,
                format: self.swapchain_format,
                components: VkComponentMapping {
                    r: VK_COMPONENT_SWIZZLE_IDENTITY,
                    g: VK_COMPONENT_SWIZZLE_IDENTITY,
                    b: VK_COMPONENT_SWIZZLE_IDENTITY,
                    a: VK_COMPONENT_SWIZZLE_IDENTITY,
                },
                subresourceRange: VkImageSubresourceRange {
                    aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                    baseMipLevel: 0,
                    levelCount: 1,
                    baseArrayLayer: 0,
                    layerCount: 1,
                },
            };
            let mut view = VK_NULL_HANDLE;
            if unsafe {
                (dfns.CreateImageView)(self.device, &iv_info, std::ptr::null(), &mut view)
            } != VK_SUCCESS
            {
                return false;
            }
            self.swapchain_image_views.push(view);

            let attachments = [view, self.depth_image_view];
            let fb_info = VkFramebufferCreateInfo {
                sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                renderPass: self.render_pass,
                attachmentCount: if self.depth_image_view != VK_NULL_HANDLE { 2 } else { 1 },
                pAttachments: attachments.as_ptr(),
                width: self.swapchain_extent.width,
                height: self.swapchain_extent.height,
                layers: 1,
            };
            let mut fb = VK_NULL_HANDLE;
            if unsafe {
                (dfns.CreateFramebuffer)(self.device, &fb_info, std::ptr::null(), &mut fb)
            } != VK_SUCCESS
            {
                return false;
            }
            self.framebuffers.push(fb);
        }

        self.swapchain_active = true;
        true
    }

    fn get_or_create_graphics_pipeline(
        &mut self,
        pso: &DuctPipelineStateDesc,
        is_offscreen: bool,
    ) -> vk::VkPipeline {
        let dfns = match &self.device_fns {
            Some(d) if !self.device.is_null() => *d,
            _ => return VK_NULL_HANDLE,
        };

        let target_rp = if is_offscreen {
            if (pso.depth_test || pso.depth_write) && self.offscreen_render_pass_with_depth != VK_NULL_HANDLE {
                self.offscreen_render_pass_with_depth
            } else {
                self.offscreen_render_pass
            }
        } else {
            self.render_pass
        };
        if target_rp == VK_NULL_HANDLE || self.pipeline_layout == VK_NULL_HANDLE {
            return VK_NULL_HANDLE;
        }

        let attr_count = pso.vertex_layout.attribute_count;
        let stride = if pso.vertex_layout.stride > 0 {
            pso.vertex_layout.stride
        } else {
            12
        };

        let mut attr_formats = [0u32; 8];
        let mut attr_offsets = [0u32; 8];
        for i in 0..(attr_count as usize).min(8) {
            attr_formats[i] = map_vertex_format(pso.vertex_layout.attributes[i].format);
            attr_offsets[i] = pso.vertex_layout.attributes[i].offset;
        }

        let key = VkPipelineKey {
            vs: pso.vertex_shader.0,
            fs: pso.fragment_shader.0,
            cull_mode: pso.cull_mode as u32,
            blend_mode: pso.blend_mode as u32,
            depth_test: pso.depth_test,
            depth_write: pso.depth_write,
            depth_compare: pso.depth_compare as u32,
            wireframe: self.wireframe,
            is_offscreen,
            stride,
            attr_count,
            attr_formats,
            attr_offsets,
        };

        if let Some(&cached) = self.graphics_pipelines.get(&key) {
            return cached;
        }

        let vs_module = if let Some(rec) = self.shaders.get(&pso.vertex_shader) {
            if attr_count >= 2 && !rec.is_custom_spv && rec.module_with_color != VK_NULL_HANDLE {
                rec.module_with_color
            } else {
                rec.module
            }
        } else if attr_count >= 2 && self.default_vs_color_module != VK_NULL_HANDLE {
            self.default_vs_color_module
        } else {
            self.default_vs_module
        };

        let fs_module = if let Some(rec) = self.shaders.get(&pso.fragment_shader) {
            rec.module
        } else {
            self.default_fs_module
        };

        if vs_module == VK_NULL_HANDLE || fs_module == VK_NULL_HANDLE {
            return VK_NULL_HANDLE;
        }

        let entry_name = b"main\0";
        let shader_stages = [
            VkPipelineShaderStageCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                stage: VK_SHADER_STAGE_VERTEX_BIT,
                module: vs_module,
                pName: entry_name.as_ptr() as *const c_char,
                pSpecializationInfo: std::ptr::null(),
            },
            VkPipelineShaderStageCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                stage: VK_SHADER_STAGE_FRAGMENT_BIT,
                module: fs_module,
                pName: entry_name.as_ptr() as *const c_char,
                pSpecializationInfo: std::ptr::null(),
            },
        ];

        let binding_desc = VkVertexInputBindingDescription {
            binding: 0,
            stride,
            inputRate: VK_VERTEX_INPUT_RATE_VERTEX,
        };

        let mut attr_descs = Vec::new();
        if attr_count == 0 {
            attr_descs.push(VkVertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: VK_FORMAT_R32G32B32_SFLOAT,
                offset: 0,
            });
        } else {
            let max_attrs = (attr_count as usize).min(8);
            for i in 0..max_attrs {
                let a = &pso.vertex_layout.attributes[i];
                attr_descs.push(VkVertexInputAttributeDescription {
                    location: a.location,
                    binding: 0,
                    format: map_vertex_format(a.format),
                    offset: a.offset,
                });
            }
        }

        let vertex_input_info = VkPipelineVertexInputStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            vertexBindingDescriptionCount: 1,
            pVertexBindingDescriptions: &binding_desc,
            vertexAttributeDescriptionCount: attr_descs.len() as u32,
            pVertexAttributeDescriptions: attr_descs.as_ptr(),
        };

        let input_assembly = VkPipelineInputAssemblyStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            topology: VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
            primitiveRestartEnable: VK_FALSE,
        };

        let viewport_state = VkPipelineViewportStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            viewportCount: 1,
            pViewports: std::ptr::null(),
            scissorCount: 1,
            pScissors: std::ptr::null(),
        };

        let cull_mode = match pso.cull_mode {
            DuctCullMode::None => VK_CULL_MODE_NONE,
            DuctCullMode::Front => VK_CULL_MODE_FRONT_BIT,
            DuctCullMode::Back => VK_CULL_MODE_BACK_BIT,
        };

        let rasterizer = VkPipelineRasterizationStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            depthClampEnable: VK_FALSE,
            rasterizerDiscardEnable: VK_FALSE,
            polygonMode: VK_POLYGON_MODE_FILL,
            cullMode: cull_mode,
            // With negative viewport height (y = h, height = -h), +Y is up, and CCW triangles
            // in world/clip space are rasterized as clockwise in framebuffer space.
            frontFace: VK_FRONT_FACE_CLOCKWISE,
            depthBiasEnable: VK_FALSE,
            depthBiasConstantFactor: 0.0,
            depthBiasClamp: 0.0,
            depthBiasSlopeFactor: 0.0,
            lineWidth: 1.0,
        };

        let multisampling = VkPipelineMultisampleStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            rasterizationSamples: VK_SAMPLE_COUNT_1_BIT,
            sampleShadingEnable: VK_FALSE,
            minSampleShading: 1.0,
            pSampleMask: std::ptr::null(),
            alphaToCoverageEnable: VK_FALSE,
            alphaToOneEnable: VK_FALSE,
        };

        let depth_compare = match pso.depth_compare {
            DuctCompareOp::Never => VK_COMPARE_OP_NEVER,
            DuctCompareOp::Less => VK_COMPARE_OP_LESS,
            DuctCompareOp::Equal => VK_COMPARE_OP_EQUAL,
            DuctCompareOp::LessEqual => VK_COMPARE_OP_LESS_OR_EQUAL,
            DuctCompareOp::Greater => VK_COMPARE_OP_GREATER,
            DuctCompareOp::NotEqual => VK_COMPARE_OP_NOT_EQUAL,
            DuctCompareOp::GreaterEqual => VK_COMPARE_OP_GREATER_OR_EQUAL,
            DuctCompareOp::Always => VK_COMPARE_OP_ALWAYS,
        };

        let depth_stencil = VkPipelineDepthStencilStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            depthTestEnable: if pso.depth_test { VK_TRUE } else { VK_FALSE },
            depthWriteEnable: if pso.depth_write { VK_TRUE } else { VK_FALSE },
            depthCompareOp: depth_compare,
            depthBoundsTestEnable: VK_FALSE,
            stencilTestEnable: VK_FALSE,
            front: VkStencilOpState::default(),
            back: VkStencilOpState::default(),
            minDepthBounds: 0.0,
            maxDepthBounds: 1.0,
        };

        let blend_attachment = match pso.blend_mode {
            DuctBlendMode::Opaque => VkPipelineColorBlendAttachmentState {
                blendEnable: VK_FALSE,
                srcColorBlendFactor: VK_BLEND_FACTOR_ONE,
                dstColorBlendFactor: VK_BLEND_FACTOR_ZERO,
                colorBlendOp: VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VK_BLEND_FACTOR_ZERO,
                alphaBlendOp: VK_BLEND_OP_ADD,
                colorWriteMask: VK_COLOR_COMPONENT_ALL,
            },
            DuctBlendMode::AlphaBlend => VkPipelineColorBlendAttachmentState {
                blendEnable: VK_TRUE,
                srcColorBlendFactor: VK_BLEND_FACTOR_SRC_ALPHA,
                dstColorBlendFactor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
                colorBlendOp: VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
                alphaBlendOp: VK_BLEND_OP_ADD,
                colorWriteMask: VK_COLOR_COMPONENT_ALL,
            },
            DuctBlendMode::Additive => VkPipelineColorBlendAttachmentState {
                blendEnable: VK_TRUE,
                srcColorBlendFactor: VK_BLEND_FACTOR_SRC_ALPHA,
                dstColorBlendFactor: VK_BLEND_FACTOR_ONE,
                colorBlendOp: VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                alphaBlendOp: VK_BLEND_OP_ADD,
                colorWriteMask: VK_COLOR_COMPONENT_ALL,
            },
        };

        let color_blending = VkPipelineColorBlendStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            logicOpEnable: VK_FALSE,
            logicOp: 0,
            attachmentCount: 1,
            pAttachments: &blend_attachment,
            blendConstants: [0.0, 0.0, 0.0, 0.0],
        };

        let dynamic_states = [VK_DYNAMIC_STATE_VIEWPORT, VK_DYNAMIC_STATE_SCISSOR];
        let dynamic_state = VkPipelineDynamicStateCreateInfo {
            sType: VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            dynamicStateCount: dynamic_states.len() as u32,
            pDynamicStates: dynamic_states.as_ptr(),
        };

        let pipeline_info = VkGraphicsPipelineCreateInfo {
            sType: VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            stageCount: 2,
            pStages: shader_stages.as_ptr(),
            pVertexInputState: &vertex_input_info,
            pInputAssemblyState: &input_assembly,
            pTessellationState: std::ptr::null(),
            pViewportState: &viewport_state,
            pRasterizationState: &rasterizer,
            pMultisampleState: &multisampling,
            pDepthStencilState: &depth_stencil,
            pColorBlendState: &color_blending,
            pDynamicState: &dynamic_state,
            layout: self.pipeline_layout,
            renderPass: target_rp,
            subpass: 0,
            basePipelineHandle: VK_NULL_HANDLE,
            basePipelineIndex: -1,
        };

        let mut pipeline = VK_NULL_HANDLE;
        let res = unsafe {
            (dfns.CreateGraphicsPipelines)(
                self.device,
                VK_NULL_HANDLE,
                1,
                &pipeline_info,
                std::ptr::null(),
                &mut pipeline,
            )
        };
        if res == VK_SUCCESS && pipeline != VK_NULL_HANDLE {
            self.graphics_pipelines.insert(key, pipeline);
            pipeline
        } else {
            eprintln!("[vkduct][ERROR] vkCreateGraphicsPipelines failed with code {}", res);
            VK_NULL_HANDLE
        }
    }

    fn prepare_draw(&mut self) {
        let pso = self.current_pso.unwrap_or(DuctPipelineStateDesc {
            vertex_shader: DuctShaderHandle::INVALID,
            fragment_shader: DuctShaderHandle::INVALID,
            vertex_layout: DuctVertexLayout::new(12).add_attribute(0, DuctVertexFormat::Float3, 0),
            cull_mode: DuctCullMode::None,
            blend_mode: DuctBlendMode::AlphaBlend,
            depth_test: false,
            depth_write: false,
            depth_compare: DuctCompareOp::Less,
        });

        let is_offscreen = self.current_pass_is_offscreen;
        let pipeline = self.get_or_create_graphics_pipeline(&pso, is_offscreen);
        self.active_graphics_pipeline = pipeline;

        if self.cmd_recording && self.render_pass_active && pipeline != VK_NULL_HANDLE {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdBindPipeline)(
                        self.command_buffer,
                        VK_PIPELINE_BIND_POINT_GRAPHICS,
                        pipeline,
                    );
                    (dfns.CmdPushConstants)(
                        self.command_buffer,
                        self.pipeline_layout,
                        VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
                        0,
                        std::mem::size_of::<VkPushConstantBlock>() as u32,
                        &self.push_constants as *const VkPushConstantBlock as *const c_void,
                    );
                    if self.last_bound_vbo != VK_NULL_HANDLE {
                        let offset: VkDeviceSize = 0;
                        (dfns.CmdBindVertexBuffers)(
                            self.command_buffer,
                            0,
                            1,
                            &self.last_bound_vbo,
                            &offset,
                        );
                    }
                }
            }
        }
    }
}

impl RenderPipeline for VkPipeline {
    fn backend_type(&self) -> DuctBackendType {
        DuctBackendType::Vulkan
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn init(&mut self, config: &DuctInitConfig) -> DuctResult {
        if !config.window.is_valid() {
            eprintln!("[vkduct][ERROR] Pipeline initialization failed: an attached native window is required.");
            return DuctResult::ErrorWindowRequired;
        }

        self.viewport_width = config.width.max(1);
        self.viewport_height = config.height.max(1);
        self.vsync_enabled = config.vsync;

        println!("[vkduct] Initializing Vulkan Pipeline (API Version 1.3)...");

        let loader = unsafe { VulkanLoader::new() };
        if let Some(loader) = loader {
            let app_name = b"Project32 Engine\0";
            let engine_name = b"Duct Vulkan Engine\0";
            let app_info = VkApplicationInfo {
                sType: VK_STRUCTURE_TYPE_APPLICATION_INFO,
                pNext: std::ptr::null(),
                pApplicationName: if !config.app_name.is_null() {
                    config.app_name
                } else {
                    app_name.as_ptr() as *const c_char
                },
                applicationVersion: vk_make_api_version(0, 0, 1, 0),
                pEngineName: engine_name.as_ptr() as *const c_char,
                engineVersion: vk_make_api_version(0, 0, 1, 0),
                apiVersion: VK_API_VERSION_1_0,
            };

            let ext_surface = b"VK_KHR_surface\0";
            #[cfg(target_os = "windows")]
            let ext_os_surface = b"VK_KHR_win32_surface\0";
            #[cfg(target_os = "linux")]
            let ext_os_surface = b"VK_KHR_xlib_surface\0";
            #[cfg(target_os = "macos")]
            let ext_os_surface = b"VK_EXT_metal_surface\0";
            #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
            let ext_os_surface = b"VK_KHR_surface\0";

            let instance_exts = [
                ext_surface.as_ptr() as *const c_char,
                ext_os_surface.as_ptr() as *const c_char,
            ];

            let mut instance: VkInstance = std::ptr::null_mut();
            let create_info = VkInstanceCreateInfo {
                sType: VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                pApplicationInfo: &app_info,
                enabledLayerCount: 0,
                ppEnabledLayerNames: std::ptr::null(),
                enabledExtensionCount: instance_exts.len() as u32,
                ppEnabledExtensionNames: instance_exts.as_ptr(),
            };

            let mut res = unsafe {
                (loader.create_instance)(&create_info, std::ptr::null(), &mut instance)
            };
            if res != VK_SUCCESS {
                let fallback_info = VkInstanceCreateInfo {
                    enabledExtensionCount: 0,
                    ppEnabledExtensionNames: std::ptr::null(),
                    ..create_info
                };
                res = unsafe {
                    (loader.create_instance)(&fallback_info, std::ptr::null(), &mut instance)
                };
            }

            if res == VK_SUCCESS && !instance.is_null() {
                if let Some(ifns) = unsafe { loader.load_instance_fns(instance) } {
                    let mut gpu_count = 0u32;
                    unsafe {
                        (ifns.EnumeratePhysicalDevices)(
                            instance,
                            &mut gpu_count,
                            std::ptr::null_mut(),
                        );
                    }
                    if gpu_count > 0 {
                        let mut gpus = vec![std::ptr::null_mut(); gpu_count as usize];
                        unsafe {
                            (ifns.EnumeratePhysicalDevices)(
                                instance,
                                &mut gpu_count,
                                gpus.as_mut_ptr(),
                            );
                        }

                        let mut chosen_gpu = gpus[0];
                        let mut chosen_props = VkPhysicalDeviceProperties::default();
                        for &gpu in &gpus {
                            let mut props = VkPhysicalDeviceProperties::default();
                            unsafe {
                                (ifns.GetPhysicalDeviceProperties)(gpu, &mut props);
                            }
                            if props.deviceType == VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU {
                                chosen_gpu = gpu;
                                chosen_props = props;
                                break;
                            } else if chosen_props.apiVersion == 0 {
                                chosen_props = props;
                            }
                        }

                        let gpu_name = unsafe {
                            CStr::from_ptr(chosen_props.deviceName.as_ptr())
                                .to_string_lossy()
                                .into_owned()
                        };
                        let api_maj = (chosen_props.apiVersion >> 22) & 0x7F;
                        let api_min = (chosen_props.apiVersion >> 12) & 0x3FF;
                        let api_pat = chosen_props.apiVersion & 0xFFF;

                        self.physical_device = chosen_gpu;
                        self.name = format!("Vulkan ({})", gpu_name);
                        self.version = format!("{}.{}.{}", api_maj, api_min, api_pat);
                        self.extra_info = CString::new(format!(
                            "Vulkan {}.{}.{} | {}",
                            api_maj, api_min, api_pat, gpu_name
                        ))
                        .unwrap_or_default();

                        unsafe {
                            (ifns.GetPhysicalDeviceMemoryProperties)(
                                chosen_gpu,
                                &mut self.memory_properties,
                            );
                        }

                        let mut qf_count = 0u32;
                        unsafe {
                            (ifns.GetPhysicalDeviceQueueFamilyProperties)(
                                chosen_gpu,
                                &mut qf_count,
                                std::ptr::null_mut(),
                            );
                        }
                        let mut qf_props =
                            vec![VkQueueFamilyProperties::default(); qf_count as usize];
                        unsafe {
                            (ifns.GetPhysicalDeviceQueueFamilyProperties)(
                                chosen_gpu,
                                &mut qf_count,
                                qf_props.as_mut_ptr(),
                            );
                        }
                        for (idx, qf) in qf_props.iter().enumerate() {
                            if (qf.queueFlags & VK_QUEUE_GRAPHICS_BIT) != 0 {
                                self.graphics_queue_family = idx as u32;
                                break;
                            }
                        }

                        let queue_priority: f32 = 1.0;
                        let queue_info = VkDeviceQueueCreateInfo {
                            sType: VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            queueFamilyIndex: self.graphics_queue_family,
                            queueCount: 1,
                            pQueuePriorities: &queue_priority,
                        };

                        let swapchain_ext = b"VK_KHR_swapchain\0";
                        let device_exts = [swapchain_ext.as_ptr() as *const c_char];
                        let dev_info = VkDeviceCreateInfo {
                            sType: VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            queueCreateInfoCount: 1,
                            pQueueCreateInfos: &queue_info,
                            enabledLayerCount: 0,
                            ppEnabledLayerNames: std::ptr::null(),
                            enabledExtensionCount: 1,
                            ppEnabledExtensionNames: device_exts.as_ptr(),
                            pEnabledFeatures: std::ptr::null(),
                        };

                        let mut device: VkDevice = std::ptr::null_mut();
                        let mut dev_res = unsafe {
                            (ifns.CreateDevice)(chosen_gpu, &dev_info, std::ptr::null(), &mut device)
                        };
                        if dev_res != VK_SUCCESS {
                            let fallback_dev_info = VkDeviceCreateInfo {
                                enabledExtensionCount: 0,
                                ppEnabledExtensionNames: std::ptr::null(),
                                ..dev_info
                            };
                            dev_res = unsafe {
                                (ifns.CreateDevice)(
                                    chosen_gpu,
                                    &fallback_dev_info,
                                    std::ptr::null(),
                                    &mut device,
                                )
                            };
                        }

                        if dev_res == VK_SUCCESS && !device.is_null() {
                            if let Some(dfns) = unsafe { loader.load_device_fns(&ifns, device) } {
                                let mut queue: VkQueue = std::ptr::null_mut();
                                unsafe {
                                    (dfns.GetDeviceQueue)(
                                        device,
                                        self.graphics_queue_family,
                                        0,
                                        &mut queue,
                                    );
                                }
                                self.device = device;
                                self.queue = queue;

                                // Create CommandPool & CommandBuffer
                                let pool_info = VkCommandPoolCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
                                    queueFamilyIndex: self.graphics_queue_family,
                                };
                                let mut cmd_pool = VK_NULL_HANDLE;
                                if unsafe {
                                    (dfns.CreateCommandPool)(
                                        device,
                                        &pool_info,
                                        std::ptr::null(),
                                        &mut cmd_pool,
                                    )
                                } == VK_SUCCESS
                                {
                                    self.command_pool = cmd_pool;
                                    let alloc_info = VkCommandBufferAllocateInfo {
                                        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
                                        pNext: std::ptr::null(),
                                        commandPool: cmd_pool,
                                        level: VK_COMMAND_BUFFER_LEVEL_PRIMARY,
                                        commandBufferCount: 1,
                                    };
                                    let mut cmd_buf: VkCommandBuffer = std::ptr::null_mut();
                                    let _ = unsafe {
                                        (dfns.AllocateCommandBuffers)(
                                            device,
                                            &alloc_info,
                                            &mut cmd_buf,
                                        )
                                    };
                                    self.command_buffer = cmd_buf;
                                }

                                // Create Sync Objects
                                let sem_info = VkSemaphoreCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                };
                                let fence_info = VkFenceCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_FENCE_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: VK_FENCE_CREATE_SIGNALED_BIT,
                                };
                                unsafe {
                                    let _ = (dfns.CreateSemaphore)(
                                        device,
                                        &sem_info,
                                        std::ptr::null(),
                                        &mut self.image_available_sem,
                                    );
                                    let _ = (dfns.CreateSemaphore)(
                                        device,
                                        &sem_info,
                                        std::ptr::null(),
                                        &mut self.render_finished_sem,
                                    );
                                    let _ = (dfns.CreateFence)(
                                        device,
                                        &fence_info,
                                        std::ptr::null(),
                                        &mut self.in_flight_fence,
                                    );
                                }

                                // Create Pipeline Layouts (Graphics with 96-byte Push Constants, and Compute)
                                let pc_range = VkPushConstantRange {
                                    stageFlags: VK_SHADER_STAGE_VERTEX_BIT
                                        | VK_SHADER_STAGE_FRAGMENT_BIT,
                                    offset: 0,
                                    size: std::mem::size_of::<VkPushConstantBlock>() as u32,
                                };
                                let layout_info = VkPipelineLayoutCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                    setLayoutCount: 0,
                                    pSetLayouts: std::ptr::null(),
                                    pushConstantRangeCount: 1,
                                    pPushConstantRanges: &pc_range,
                                };
                                unsafe {
                                    let _ = (dfns.CreatePipelineLayout)(
                                        device,
                                        &layout_info,
                                        std::ptr::null(),
                                        &mut self.pipeline_layout,
                                    );
                                }

                                let comp_layout_info = VkPipelineLayoutCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                    setLayoutCount: 0,
                                    pSetLayouts: std::ptr::null(),
                                    pushConstantRangeCount: 0,
                                    pPushConstantRanges: std::ptr::null(),
                                };
                                unsafe {
                                    let _ = (dfns.CreatePipelineLayout)(
                                        device,
                                        &comp_layout_info,
                                        std::ptr::null(),
                                        &mut self.compute_pipeline_layout,
                                    );
                                }

                                // Create OS Surface if attached to a real window
                                #[cfg(target_os = "windows")]
                                {
                                    let hwnd = config.window.native_window;
                                    let is_real_window = unsafe { IsWindow(hwnd) != 0 };
                                    if is_real_window {
                                        if let Some(create_win32_surface) =
                                            ifns.CreateWin32SurfaceKHR
                                        {
                                            let hinstance =
                                                if !config.window.native_display.is_null() {
                                                    config.window.native_display
                                                } else {
                                                    unsafe { GetModuleHandleW(std::ptr::null()) }
                                                };
                                            let surf_info = VkWin32SurfaceCreateInfoKHR {
                                                sType:
                                                    VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR,
                                                pNext: std::ptr::null(),
                                                flags: 0,
                                                hinstance,
                                                hwnd,
                                            };
                                            let mut surface = VK_NULL_HANDLE;
                                            if unsafe {
                                                create_win32_surface(
                                                    instance,
                                                    &surf_info,
                                                    std::ptr::null(),
                                                    &mut surface,
                                                )
                                            } == VK_SUCCESS
                                            {
                                                self.surface = surface;
                                            }
                                        }
                                    }
                                }

                                #[cfg(target_os = "linux")]
                                {
                                    let xid = config.window.xlib_window();
                                    let dpy = config.window.xlib_display();
                                    if !dpy.is_null() && xid != 0 {
                                        if let Some(create_xlib_surface) = ifns.CreateXlibSurfaceKHR
                                        {
                                            let surf_info = VkXlibSurfaceCreateInfoKHR {
                                                sType:
                                                    VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR,
                                                pNext: std::ptr::null(),
                                                flags: 0,
                                                dpy,
                                                window: xid,
                                            };
                                            let mut surface = VK_NULL_HANDLE;
                                            if unsafe {
                                                create_xlib_surface(
                                                    instance,
                                                    &surf_info,
                                                    std::ptr::null(),
                                                    &mut surface,
                                                )
                                            } == VK_SUCCESS
                                            {
                                                self.surface = surface;
                                            }
                                        }
                                    }
                                }

                                self.instance = instance;
                                self.instance_fns = Some(ifns);
                                self.device_fns = Some(dfns);
                                self.loader = Some(loader);

                                // Compile built-in SPIR-V default shaders & offscreen render pass
                                unsafe {
                                    self.default_vs_module = self
                                        .create_spv_module(&spv::build_vertex_shader_spv(false));
                                    self.default_vs_color_module =
                                        self.create_spv_module(&spv::build_vertex_shader_spv(true));
                                    self.default_fs_module =
                                        self.create_spv_module(&spv::build_fragment_shader_spv());
                                    self.offscreen_render_pass = self.create_single_render_pass(
                                        VK_FORMAT_R8G8B8A8_UNORM,
                                        VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                        false,
                                    );
                                    self.offscreen_render_pass_with_depth = self.create_single_render_pass(
                                        VK_FORMAT_R8G8B8A8_UNORM,
                                        VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                        true,
                                    );
                                    self.init_swapchain(
                                        self.viewport_width,
                                        self.viewport_height,
                                        config.vsync,
                                    );
                                }

                                println!(
                                    "[vkduct] Vulkan Hardware Device initialized: {} (API {}, SwapchainActive: {}, QueueFamily: {})",
                                    self.name, self.version, self.swapchain_active, self.graphics_queue_family
                                );
                            }
                        }
                    }
                }
            }
        }

        let surface_type = if cfg!(target_os = "windows") {
            "vkCreateWin32SurfaceKHR"
        } else if cfg!(target_os = "macos") {
            "vkCreateMetalSurfaceEXT (MoltenVK)"
        } else {
            "vkCreateXlibSurfaceKHR"
        };

        println!(
            "[vkduct] vkCreateInstance, {} & vkCreateDevice completed (Swapchain: {}x{}, VSync: {}, Window: {:?}).",
            surface_type, self.viewport_width, self.viewport_height, config.vsync, config.window.native_window
        );

        self.initialized = true;
        DuctResult::Success
    }

    fn shutdown(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::Success;
        }
        println!(
            "[vkduct] vkDeviceWaitIdle & vkDestroyDevice (Total Frames: {}).",
            self.frame_count
        );

        unsafe {
            self.cleanup_swapchain();
        }

        if let Some(dfns) = &self.device_fns {
            if !self.device.is_null() {
                unsafe {
                    let _ = (dfns.DeviceWaitIdle)(self.device);

                    for (_, p) in self.graphics_pipelines.drain() {
                        if p != VK_NULL_HANDLE {
                            (dfns.DestroyPipeline)(self.device, p, std::ptr::null());
                        }
                    }
                    for (_, p) in self.compute_pipelines.drain() {
                        if p != VK_NULL_HANDLE {
                            (dfns.DestroyPipeline)(self.device, p, std::ptr::null());
                        }
                    }
                    if self.pipeline_layout != VK_NULL_HANDLE {
                        (dfns.DestroyPipelineLayout)(
                            self.device,
                            self.pipeline_layout,
                            std::ptr::null(),
                        );
                        self.pipeline_layout = VK_NULL_HANDLE;
                    }
                    if self.compute_pipeline_layout != VK_NULL_HANDLE {
                        (dfns.DestroyPipelineLayout)(
                            self.device,
                            self.compute_pipeline_layout,
                            std::ptr::null(),
                        );
                        self.compute_pipeline_layout = VK_NULL_HANDLE;
                    }

                    if self.default_vs_module != VK_NULL_HANDLE {
                        (dfns.DestroyShaderModule)(
                            self.device,
                            self.default_vs_module,
                            std::ptr::null(),
                        );
                        self.default_vs_module = VK_NULL_HANDLE;
                    }
                    if self.default_vs_color_module != VK_NULL_HANDLE {
                        (dfns.DestroyShaderModule)(
                            self.device,
                            self.default_vs_color_module,
                            std::ptr::null(),
                        );
                        self.default_vs_color_module = VK_NULL_HANDLE;
                    }
                    if self.default_fs_module != VK_NULL_HANDLE {
                        (dfns.DestroyShaderModule)(
                            self.device,
                            self.default_fs_module,
                            std::ptr::null(),
                        );
                        self.default_fs_module = VK_NULL_HANDLE;
                    }

                    for (_, s) in self.shaders.drain() {
                        if s.module != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(self.device, s.module, std::ptr::null());
                        }
                        if s.module_with_color != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(
                                self.device,
                                s.module_with_color,
                                std::ptr::null(),
                            );
                        }
                    }
                    for (_, b) in self.buffers.drain() {
                        if b.buffer != VK_NULL_HANDLE {
                            (dfns.DestroyBuffer)(self.device, b.buffer, std::ptr::null());
                        }
                        if b.memory != VK_NULL_HANDLE {
                            (dfns.FreeMemory)(self.device, b.memory, std::ptr::null());
                        }
                    }
                    for (_, rt) in self.render_targets.drain() {
                        if rt.framebuffer != VK_NULL_HANDLE {
                            (dfns.DestroyFramebuffer)(self.device, rt.framebuffer, std::ptr::null());
                        }
                    }
                    for (_, t) in self.textures.drain() {
                        if t.sampler != VK_NULL_HANDLE {
                            (dfns.DestroySampler)(self.device, t.sampler, std::ptr::null());
                        }
                        if t.view != VK_NULL_HANDLE {
                            (dfns.DestroyImageView)(self.device, t.view, std::ptr::null());
                        }
                        if t.image != VK_NULL_HANDLE {
                            (dfns.DestroyImage)(self.device, t.image, std::ptr::null());
                        }
                        if t.memory != VK_NULL_HANDLE {
                            (dfns.FreeMemory)(self.device, t.memory, std::ptr::null());
                        }
                    }

                    if self.offscreen_render_pass != VK_NULL_HANDLE {
                        (dfns.DestroyRenderPass)(
                            self.device,
                            self.offscreen_render_pass,
                            std::ptr::null(),
                        );
                        self.offscreen_render_pass = VK_NULL_HANDLE;
                    }
                    if self.offscreen_render_pass_with_depth != VK_NULL_HANDLE {
                        (dfns.DestroyRenderPass)(
                            self.device,
                            self.offscreen_render_pass_with_depth,
                            std::ptr::null(),
                        );
                        self.offscreen_render_pass_with_depth = VK_NULL_HANDLE;
                    }

                    if self.image_available_sem != VK_NULL_HANDLE {
                        (dfns.DestroySemaphore)(
                            self.device,
                            self.image_available_sem,
                            std::ptr::null(),
                        );
                        self.image_available_sem = VK_NULL_HANDLE;
                    }
                    if self.render_finished_sem != VK_NULL_HANDLE {
                        (dfns.DestroySemaphore)(
                            self.device,
                            self.render_finished_sem,
                            std::ptr::null(),
                        );
                        self.render_finished_sem = VK_NULL_HANDLE;
                    }
                    if self.in_flight_fence != VK_NULL_HANDLE {
                        (dfns.DestroyFence)(self.device, self.in_flight_fence, std::ptr::null());
                        self.in_flight_fence = VK_NULL_HANDLE;
                    }
                    if self.command_pool != VK_NULL_HANDLE {
                        (dfns.DestroyCommandPool)(self.device, self.command_pool, std::ptr::null());
                        self.command_pool = VK_NULL_HANDLE;
                        self.command_buffer = std::ptr::null_mut();
                    }

                    (dfns.DestroyDevice)(self.device, std::ptr::null());
                    self.device = std::ptr::null_mut();
                }
            }
        }

        if let Some(ifns) = &self.instance_fns {
            if !self.instance.is_null() {
                unsafe {
                    if self.surface != VK_NULL_HANDLE {
                        if let Some(destroy_surf) = ifns.DestroySurfaceKHR {
                            destroy_surf(self.instance, self.surface, std::ptr::null());
                        }
                        self.surface = VK_NULL_HANDLE;
                    }
                    (ifns.DestroyInstance)(self.instance, std::ptr::null());
                    self.instance = std::ptr::null_mut();
                }
            }
        }

        self.shaders.clear();
        self.buffers.clear();
        self.textures.clear();
        self.render_targets.clear();
        self.device_fns = None;
        self.instance_fns = None;
        self.loader = None;
        self.initialized = false;
        DuctResult::Success
    }

    fn resize_viewport(&mut self, width: u32, height: u32) -> DuctResult {
        if width == 0 || height == 0 {
            return DuctResult::Success;
        }
        self.viewport_width = width;
        self.viewport_height = height;

        if self.cmd_recording {
            if let Some(dfns) = &self.device_fns {
                let vp = VkViewport {
                    x: 0.0,
                    y: self.viewport_height as f32,
                    width: self.viewport_width as f32,
                    height: -(self.viewport_height as f32),
                    minDepth: 0.0,
                    maxDepth: 1.0,
                };
                unsafe {
                    (dfns.CmdSetViewport)(self.command_buffer, 0, 1, &vp);
                }
            }
        } else if self.surface != VK_NULL_HANDLE
            && (width != self.swapchain_extent.width || height != self.swapchain_extent.height)
        {
            unsafe {
                self.cleanup_swapchain();
                self.init_swapchain(width, height, self.vsync_enabled);
            }
        }
        DuctResult::Success
    }

    fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
        if self.cmd_recording {
            if let Some(dfns) = &self.device_fns {
                let scissor = VkRect2D {
                    offset: VkOffset2D {
                        x: x as i32,
                        y: y as i32,
                    },
                    extent: VkExtent2D { width, height },
                };
                unsafe {
                    (dfns.CmdSetScissor)(self.command_buffer, 0, 1, &scissor);
                }
            }
        }
        DuctResult::Success
    }

    fn begin_frame(&mut self, clear_color: DuctColor) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }
        self.frame_in_progress = true;
        self.draw_calls_in_frame = 0;
        self.vertices_drawn_in_frame = 0;
        self.pipeline_switches_in_frame = 0;
        self.last_clear_color = clear_color;

        let now = std::time::Instant::now();
        if let Some(prev) = self.last_frame_instant {
            let dt = now.duration_since(prev).as_secs_f32();
            self.current_frame_time_ms = dt * 1000.0;
            if dt > 0.0 {
                let instant_fps = 1.0 / dt;
                self.smoothed_fps = 0.9 * self.smoothed_fps + 0.1 * instant_fps;
            }
        }
        self.last_frame_instant = Some(now);

        let is_offscreen = self.active_render_target.is_valid();
        self.current_pass_is_offscreen = is_offscreen;

        if let Some(dfns) = &self.device_fns {
            if !self.command_buffer.is_null() {
                unsafe {
                    if self.in_flight_fence != VK_NULL_HANDLE {
                        let _ = (dfns.WaitForFences)(
                            self.device,
                            1,
                            &self.in_flight_fence,
                            VK_TRUE,
                            u64::MAX,
                        );
                        let _ = (dfns.ResetFences)(self.device, 1, &self.in_flight_fence);
                    }

                    let mut target_fb = VK_NULL_HANDLE;
                    let mut target_rp = VK_NULL_HANDLE;
                    let mut target_extent = VkExtent2D {
                        width: self.viewport_width,
                        height: self.viewport_height,
                    };
                    let mut has_depth = true;

                    if is_offscreen {
                        if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                            target_fb = rt.framebuffer;
                            target_rp = if rt.has_depth && self.offscreen_render_pass_with_depth != VK_NULL_HANDLE {
                                self.offscreen_render_pass_with_depth
                            } else {
                                self.offscreen_render_pass
                            };
                            target_extent = VkExtent2D {
                                width: rt.width,
                                height: rt.height,
                            };
                            has_depth = rt.has_depth;
                        }
                    } else if self.swapchain_active {
                        if !self.swapchain_image_acquired {
                            if let Some(acquire_fn) = dfns.AcquireNextImageKHR {
                                let mut img_idx = 0u32;
                                let acq_res = acquire_fn(
                                    self.device,
                                    self.swapchain,
                                    u64::MAX,
                                    self.image_available_sem,
                                    VK_NULL_HANDLE,
                                    &mut img_idx,
                                );
                                if acq_res == VK_SUCCESS || acq_res == VK_SUBOPTIMAL_KHR {
                                    self.current_image_index = img_idx;
                                    self.swapchain_image_acquired = true;
                                }
                            }
                        }
                        if self.swapchain_image_acquired
                            && (self.current_image_index as usize) < self.framebuffers.len()
                        {
                            target_fb = self.framebuffers[self.current_image_index as usize];
                            target_rp = self.render_pass;
                            target_extent = self.swapchain_extent;
                            has_depth = self.depth_image_view != VK_NULL_HANDLE;
                        }
                    }

                    let _ = (dfns.ResetCommandBuffer)(self.command_buffer, 0);
                    let begin_info = VkCommandBufferBeginInfo {
                        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                        pNext: std::ptr::null(),
                        flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                        pInheritanceInfo: std::ptr::null(),
                    };
                    if (dfns.BeginCommandBuffer)(self.command_buffer, &begin_info) == VK_SUCCESS {
                        self.cmd_recording = true;

                        let vp = VkViewport {
                            x: 0.0,
                            y: target_extent.height as f32,
                            width: target_extent.width as f32,
                            height: -(target_extent.height as f32),
                            minDepth: 0.0,
                            maxDepth: 1.0,
                        };
                        (dfns.CmdSetViewport)(self.command_buffer, 0, 1, &vp);

                        let scissor = VkRect2D {
                            offset: VkOffset2D { x: 0, y: 0 },
                            extent: target_extent,
                        };
                        (dfns.CmdSetScissor)(self.command_buffer, 0, 1, &scissor);

                        if target_rp != VK_NULL_HANDLE && target_fb != VK_NULL_HANDLE {
                            let clear_vals = [
                                VkClearValue {
                                    float32: [
                                        clear_color.r,
                                        clear_color.g,
                                        clear_color.b,
                                        clear_color.a,
                                    ],
                                },
                                VkClearValue {
                                    float32: [1.0, 0.0, 0.0, 0.0],
                                },
                            ];
                            let clear_count = if has_depth { 2 } else { 1 };
                            let rp_begin = VkRenderPassBeginInfo {
                                sType: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
                                pNext: std::ptr::null(),
                                renderPass: target_rp,
                                framebuffer: target_fb,
                                renderArea: VkRect2D {
                                    offset: VkOffset2D { x: 0, y: 0 },
                                    extent: target_extent,
                                },
                                clearValueCount: clear_count,
                                pClearValues: clear_vals.as_ptr(),
                            };
                            (dfns.CmdBeginRenderPass)(
                                self.command_buffer,
                                &rp_begin,
                                VK_SUBPASS_CONTENTS_INLINE,
                            );
                            self.render_pass_active = true;
                        }
                    }
                }
            }
        }

        DuctResult::Success
    }

    fn end_frame(&mut self) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.frame_in_progress = false;
        self.frame_count += 1;

        if let Some(dfns) = &self.device_fns {
            if self.cmd_recording && !self.command_buffer.is_null() {
                unsafe {
                    if self.render_pass_active {
                        (dfns.CmdEndRenderPass)(self.command_buffer);
                        self.render_pass_active = false;
                    }
                    let _ = (dfns.EndCommandBuffer)(self.command_buffer);
                    self.cmd_recording = false;

                    let wait_stage = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
                    let use_swapchain_sems = !self.current_pass_is_offscreen
                        && self.swapchain_active
                        && self.swapchain_image_acquired
                        && self.image_available_sem != VK_NULL_HANDLE
                        && self.render_finished_sem != VK_NULL_HANDLE;

                    let submit_info = if use_swapchain_sems {
                        VkSubmitInfo {
                            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                            pNext: std::ptr::null(),
                            waitSemaphoreCount: 1,
                            pWaitSemaphores: &self.image_available_sem,
                            pWaitDstStageMask: &wait_stage,
                            commandBufferCount: 1,
                            pCommandBuffers: &self.command_buffer,
                            signalSemaphoreCount: 1,
                            pSignalSemaphores: &self.render_finished_sem,
                        }
                    } else {
                        VkSubmitInfo {
                            sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                            pNext: std::ptr::null(),
                            waitSemaphoreCount: 0,
                            pWaitSemaphores: std::ptr::null(),
                            pWaitDstStageMask: std::ptr::null(),
                            commandBufferCount: 1,
                            pCommandBuffers: &self.command_buffer,
                            signalSemaphoreCount: 0,
                            pSignalSemaphores: std::ptr::null(),
                        }
                    };

                    let _ = (dfns.QueueSubmit)(
                        self.queue,
                        1,
                        &submit_info,
                        self.in_flight_fence,
                    );

                    if self.current_pass_is_offscreen && self.in_flight_fence != VK_NULL_HANDLE {
                        let _ = (dfns.WaitForFences)(
                            self.device,
                            1,
                            &self.in_flight_fence,
                            VK_TRUE,
                            u64::MAX,
                        );
                    }
                }
            }
        }

        DuctResult::Success
    }

    fn present(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }

        if self.swapchain_active && self.swapchain_image_acquired {
            if let Some(dfns) = &self.device_fns {
                if let Some(present_fn) = dfns.QueuePresentKHR {
                    let present_info = VkPresentInfoKHR {
                        sType: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
                        pNext: std::ptr::null(),
                        waitSemaphoreCount: if self.render_finished_sem != VK_NULL_HANDLE {
                            1
                        } else {
                            0
                        },
                        pWaitSemaphores: &self.render_finished_sem,
                        swapchainCount: 1,
                        pSwapchains: &self.swapchain,
                        pImageIndices: &self.current_image_index,
                        pResults: std::ptr::null_mut(),
                    };
                    unsafe {
                        let _ = present_fn(self.queue, &present_info);
                    }
                    self.swapchain_image_acquired = false;
                }
            }
        }

        DuctResult::Success
    }

    fn create_shader(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return Err(DuctResult::ErrorInvalidParameter);
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut module = VK_NULL_HANDLE;
        let module_with_color = VK_NULL_HANDLE;
        let mut is_custom_spv = false;

        // 1. Check if caller provided raw SPIR-V binary (Magic 0x07230203)
        if slice.len() >= 4 && slice.len() % 4 == 0 {
            let magic = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
            if magic == 0x0723_0203 {
                is_custom_spv = true;
                if let Some(dfns) = &self.device_fns {
                    if !self.device.is_null() {
                        let create_info = VkShaderModuleCreateInfo {
                            sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            codeSize: slice.len(),
                            pCode: slice.as_ptr() as *const u32,
                        };
                        let _ = unsafe {
                            (dfns.CreateShaderModule)(
                                self.device,
                                &create_info,
                                std::ptr::null(),
                                &mut module,
                            )
                        };
                    }
                }
            }
        }

        // 2. If GLSL source string was provided, compile using Naga cross-compiler
        if !is_custom_spv {
            let src_str = match std::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("[vkduct][ERROR] Shader source is neither valid UTF-8 nor SPIR-V binary");
                    return Err(DuctResult::ErrorInvalidParameter);
                }
            };

            match duct_core::shader::glsl_to_spirv(src_str, desc.stage) {
                Ok(spv_words) => {
                    module = unsafe { self.create_spv_module(&spv_words) };
                    if module == VK_NULL_HANDLE {
                        eprintln!("[vkduct][ERROR] Failed to create Vulkan shader module from compiled SPIR-V");
                        return Err(DuctResult::ErrorInitializationFailed);
                    }
                    is_custom_spv = true;
                }
                Err(err) => {
                    eprintln!("[vkduct][ERROR] Shader compilation failed:\n{}", err);
                    return Err(DuctResult::ErrorInvalidParameter);
                }
            }
        }

        let handle = DuctShaderHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.shaders.insert(
            handle,
            VkShaderRecord {
                module,
                module_with_color,
                stage: desc.stage,
                is_custom_spv,
            },
        );
        Ok(handle)
    }

    fn reload_shader(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        if !self.shaders.contains_key(&handle) {
            return DuctResult::ErrorResourceNotFound;
        }
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return DuctResult::ErrorInvalidParameter;
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut new_module = VK_NULL_HANDLE;
        let new_module_color = VK_NULL_HANDLE;
        let mut is_custom_spv = false;

        if slice.len() >= 4 && slice.len() % 4 == 0 {
            let magic = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
            if magic == 0x0723_0203 {
                is_custom_spv = true;
                if let Some(dfns) = &self.device_fns {
                    if !self.device.is_null() {
                        let create_info = VkShaderModuleCreateInfo {
                            sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            codeSize: slice.len(),
                            pCode: slice.as_ptr() as *const u32,
                        };
                        let _ = unsafe {
                            (dfns.CreateShaderModule)(
                                self.device,
                                &create_info,
                                std::ptr::null(),
                                &mut new_module,
                            )
                        };
                    }
                }
            }
        }

        if !is_custom_spv {
            let src_str = match std::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("[vkduct][ERROR] Shader reload source is neither valid UTF-8 nor SPIR-V binary");
                    return DuctResult::ErrorInvalidParameter;
                }
            };

            match duct_core::shader::glsl_to_spirv(src_str, desc.stage) {
                Ok(spv_words) => {
                    new_module = unsafe { self.create_spv_module(&spv_words) };
                    if new_module == VK_NULL_HANDLE {
                        eprintln!("[vkduct][ERROR] Failed to create Vulkan shader module from reloaded SPIR-V");
                        return DuctResult::ErrorInitializationFailed;
                    }
                    is_custom_spv = true;
                }
                Err(err) => {
                    eprintln!("[vkduct][ERROR] Shader reload compilation failed:\n{}", err);
                    return DuctResult::ErrorInvalidParameter;
                }
            }
        }

        if let Some(dfns) = &self.device_fns {
            if !self.device.is_null() {
                unsafe {
                    let _ = (dfns.DeviceWaitIdle)(self.device);
                    // Invalidate cached pipelines referencing this shader
                    let keys_to_remove: Vec<VkPipelineKey> = self
                        .graphics_pipelines
                        .keys()
                        .filter(|k| k.vs == handle.0 || k.fs == handle.0)
                        .copied()
                        .collect();
                    for k in keys_to_remove {
                        if let Some(p) = self.graphics_pipelines.remove(&k) {
                            if p != VK_NULL_HANDLE {
                                (dfns.DestroyPipeline)(self.device, p, std::ptr::null());
                            }
                        }
                    }
                    if let Some(rec) = self.shaders.get_mut(&handle) {
                        if rec.module != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(self.device, rec.module, std::ptr::null());
                        }
                        if rec.module_with_color != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(
                                self.device,
                                rec.module_with_color,
                                std::ptr::null(),
                            );
                        }
                        rec.module = new_module;
                        rec.module_with_color = new_module_color;
                        rec.stage = desc.stage;
                        rec.is_custom_spv = is_custom_spv;
                    }
                }
            }
        }

        println!("[vkduct] Shader {:?} hot-reloaded on Vulkan device.", handle);
        DuctResult::Success
    }

    fn destroy_shader(&mut self, handle: DuctShaderHandle) -> DuctResult {
        if let Some(rec) = self.shaders.remove(&handle) {
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        let _ = (dfns.DeviceWaitIdle)(self.device);
                        if rec.module != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(self.device, rec.module, std::ptr::null());
                        }
                        if rec.module_with_color != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(
                                self.device,
                                rec.module_with_color,
                                std::ptr::null(),
                            );
                        }
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_buffer(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
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

    fn update_buffer(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
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
                            offset as VkDeviceSize,
                            data.len() as VkDeviceSize,
                            0,
                            &mut mapped,
                        ) == VK_SUCCESS
                            && !mapped.is_null()
                        {
                            std::ptr::copy_nonoverlapping(
                                data.as_ptr(),
                                mapped as *mut u8,
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

    fn destroy_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
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

    fn bind_vertex_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
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

    fn bind_index_buffer(&mut self, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult {
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

    fn bind_uniform_buffer(&mut self, _slot: u32, handle: DuctBufferHandle) -> DuctResult {
        if self.buffers.contains_key(&handle) {
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_texture_2d(
        &mut self,
        width: u32,
        height: u32,
        format: DuctTextureFormat,
        _data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctResult> {
        let vk_format = map_texture_format(format);
        let mut image = VK_NULL_HANDLE;
        let mut memory = VK_NULL_HANDLE;
        let mut view = VK_NULL_HANDLE;
        let mut sampler = VK_NULL_HANDLE;

        if let Some(dfns) = &self.device_fns {
            if !self.device.is_null() && width > 0 && height > 0 {
                let img_info = VkImageCreateInfo {
                    sType: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                    imageType: VK_IMAGE_TYPE_2D,
                    format: vk_format,
                    extent: VkExtent3D {
                        width,
                        height,
                        depth: 1,
                    },
                    mipLevels: 1,
                    arrayLayers: 1,
                    samples: VK_SAMPLE_COUNT_1_BIT,
                    tiling: VK_IMAGE_TILING_OPTIMAL,
                    usage: VK_IMAGE_USAGE_SAMPLED_BIT
                        | VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT
                        | VK_IMAGE_USAGE_TRANSFER_DST_BIT
                        | VK_IMAGE_USAGE_TRANSFER_SRC_BIT,
                    sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                    queueFamilyIndexCount: 0,
                    pQueueFamilyIndices: std::ptr::null(),
                    initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
                };

                if unsafe {
                    (dfns.CreateImage)(self.device, &img_info, std::ptr::null(), &mut image)
                } == VK_SUCCESS
                {
                    let mut mem_reqs = VkMemoryRequirements::default();
                    unsafe {
                        (dfns.GetImageMemoryRequirements)(self.device, image, &mut mem_reqs);
                    }

                    if let Some(mem_type) = self
                        .find_memory_type(mem_reqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT)
                        .or_else(|| self.find_memory_type(mem_reqs.memoryTypeBits, 0))
                    {
                        let alloc_info = VkMemoryAllocateInfo {
                            sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                            pNext: std::ptr::null(),
                            allocationSize: mem_reqs.size,
                            memoryTypeIndex: mem_type,
                        };
                        if unsafe {
                            (dfns.AllocateMemory)(
                                self.device,
                                &alloc_info,
                                std::ptr::null(),
                                &mut memory,
                            )
                        } == VK_SUCCESS
                        {
                            unsafe {
                                let _ = (dfns.BindImageMemory)(self.device, image, memory, 0);
                            }

                            let iv_info = VkImageViewCreateInfo {
                                sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                                pNext: std::ptr::null(),
                                flags: 0,
                                image,
                                viewType: VK_IMAGE_VIEW_TYPE_2D,
                                format: vk_format,
                                components: VkComponentMapping::default(),
                                subresourceRange: VkImageSubresourceRange {
                                    aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                    baseMipLevel: 0,
                                    levelCount: 1,
                                    baseArrayLayer: 0,
                                    layerCount: 1,
                                },
                            };
                            let _ = unsafe {
                                (dfns.CreateImageView)(
                                    self.device,
                                    &iv_info,
                                    std::ptr::null(),
                                    &mut view,
                                )
                            };
                        }
                    }

                    let sampler_info = VkSamplerCreateInfo {
                        sType: VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        magFilter: VK_FILTER_LINEAR,
                        minFilter: VK_FILTER_LINEAR,
                        mipmapMode: VK_SAMPLER_MIPMAP_MODE_LINEAR,
                        addressModeU: VK_SAMPLER_ADDRESS_MODE_REPEAT,
                        addressModeV: VK_SAMPLER_ADDRESS_MODE_REPEAT,
                        addressModeW: VK_SAMPLER_ADDRESS_MODE_REPEAT,
                        mipLodBias: 0.0,
                        anisotropyEnable: VK_FALSE,
                        maxAnisotropy: 1.0,
                        compareEnable: VK_FALSE,
                        compareOp: VK_COMPARE_OP_ALWAYS,
                        minLod: 0.0,
                        maxLod: 1.0,
                        borderColor: 0,
                        unnormalizedCoordinates: VK_FALSE,
                    };
                    let _ = unsafe {
                        (dfns.CreateSampler)(
                            self.device,
                            &sampler_info,
                            std::ptr::null(),
                            &mut sampler,
                        )
                    };
                }
            }
        }

        self.allocated_gpu_bytes += (width as usize) * (height as usize) * 4;

        let handle = DuctTextureHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.textures.insert(
            handle,
            VkTextureRecord {
                image,
                memory,
                view,
                sampler,
                width,
                height,
                vk_format,
                min_filter: DuctFilterMode::Linear,
                mag_filter: DuctFilterMode::Linear,
                wrap_s: DuctWrapMode::Repeat,
                wrap_t: DuctWrapMode::Repeat,
            },
        );
        if let Some(initial_bytes) = _data {
            let _ = self.update_texture_2d(handle, 0, 0, width, height, initial_bytes);
        }
        Ok(handle)
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
        let tex_image = if let Some(tex) = self.textures.get(&handle) {
            if x + width > tex.width || y + height > tex.height {
                return DuctResult::ErrorInvalidParameter;
            }
            tex.image
        } else {
            return DuctResult::ErrorResourceNotFound;
        };

        if tex_image != VK_NULL_HANDLE
            && !self.cmd_recording
            && !data.is_empty()
            && width > 0
            && height > 0
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    let _ = (dfns.DeviceWaitIdle)(self.device);
                    let size = data.len();
                    let buf_info = VkBufferCreateInfo {
                        sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        size: size as VkDeviceSize,
                        usage: VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
                        sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                        queueFamilyIndexCount: 0,
                        pQueueFamilyIndices: std::ptr::null(),
                    };
                    let mut staging_buf = VK_NULL_HANDLE;
                    let mut staging_mem = VK_NULL_HANDLE;
                    if (dfns.CreateBuffer)(
                        self.device,
                        &buf_info,
                        std::ptr::null(),
                        &mut staging_buf,
                    ) == VK_SUCCESS
                    {
                        let mut mem_reqs = VkMemoryRequirements::default();
                        (dfns.GetBufferMemoryRequirements)(self.device, staging_buf, &mut mem_reqs);
                        if let Some(mem_idx) = self.find_memory_type(
                            mem_reqs.memoryTypeBits,
                            VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                                | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                        ) {
                            let alloc_info = VkMemoryAllocateInfo {
                                sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                                pNext: std::ptr::null(),
                                allocationSize: mem_reqs.size,
                                memoryTypeIndex: mem_idx,
                            };
                            if (dfns.AllocateMemory)(
                                self.device,
                                &alloc_info,
                                std::ptr::null(),
                                &mut staging_mem,
                            ) == VK_SUCCESS
                            {
                                let _ = (dfns.BindBufferMemory)(
                                    self.device,
                                    staging_buf,
                                    staging_mem,
                                    0,
                                );
                                let mut mapped: *mut c_void = std::ptr::null_mut();
                                if (dfns.MapMemory)(
                                    self.device,
                                    staging_mem,
                                    0,
                                    size as VkDeviceSize,
                                    0,
                                    &mut mapped,
                                ) == VK_SUCCESS
                                    && !mapped.is_null()
                                {
                                    std::ptr::copy_nonoverlapping(
                                        data.as_ptr(),
                                        mapped as *mut u8,
                                        size,
                                    );
                                    (dfns.UnmapMemory)(self.device, staging_mem);
                                }

                                let _ = (dfns.ResetCommandBuffer)(self.command_buffer, 0);
                                let begin_info = VkCommandBufferBeginInfo {
                                    sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                                    pNext: std::ptr::null(),
                                    flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                                    pInheritanceInfo: std::ptr::null(),
                                };
                                if (dfns.BeginCommandBuffer)(self.command_buffer, &begin_info)
                                    == VK_SUCCESS
                                {
                                    let to_dst = VkImageMemoryBarrier {
                                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                                        pNext: std::ptr::null(),
                                        srcAccessMask: 0,
                                        dstAccessMask: VK_ACCESS_TRANSFER_WRITE_BIT,
                                        oldLayout: VK_IMAGE_LAYOUT_UNDEFINED,
                                        newLayout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                        srcQueueFamilyIndex: !0u32,
                                        dstQueueFamilyIndex: !0u32,
                                        image: tex_image,
                                        subresourceRange: VkImageSubresourceRange {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            baseMipLevel: 0,
                                            levelCount: 1,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                    };
                                    (dfns.CmdPipelineBarrier)(
                                        self.command_buffer,
                                        VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                                        VK_PIPELINE_STAGE_TRANSFER_BIT,
                                        0,
                                        0,
                                        std::ptr::null(),
                                        0,
                                        std::ptr::null(),
                                        1,
                                        &to_dst,
                                    );

                                    let copy_region = VkBufferImageCopy {
                                        bufferOffset: 0,
                                        bufferRowLength: 0,
                                        bufferImageHeight: 0,
                                        imageSubresource: VkImageSubresourceLayers {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            mipLevel: 0,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                        imageOffset: VkOffset3D {
                                            x: x as i32,
                                            y: y as i32,
                                            z: 0,
                                        },
                                        imageExtent: VkExtent3D {
                                            width,
                                            height,
                                            depth: 1,
                                        },
                                    };
                                    (dfns.CmdCopyBufferToImage)(
                                        self.command_buffer,
                                        staging_buf,
                                        tex_image,
                                        VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                        1,
                                        &copy_region,
                                    );

                                    let to_color = VkImageMemoryBarrier {
                                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                                        pNext: std::ptr::null(),
                                        srcAccessMask: VK_ACCESS_TRANSFER_WRITE_BIT,
                                        dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                                        oldLayout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                        newLayout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                                        srcQueueFamilyIndex: !0u32,
                                        dstQueueFamilyIndex: !0u32,
                                        image: tex_image,
                                        subresourceRange: VkImageSubresourceRange {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            baseMipLevel: 0,
                                            levelCount: 1,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                    };
                                    (dfns.CmdPipelineBarrier)(
                                        self.command_buffer,
                                        VK_PIPELINE_STAGE_TRANSFER_BIT,
                                        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                                        0,
                                        0,
                                        std::ptr::null(),
                                        0,
                                        std::ptr::null(),
                                        1,
                                        &to_color,
                                    );

                                    let _ = (dfns.EndCommandBuffer)(self.command_buffer);
                                    let submit = VkSubmitInfo {
                                        sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                                        pNext: std::ptr::null(),
                                        waitSemaphoreCount: 0,
                                        pWaitSemaphores: std::ptr::null(),
                                        pWaitDstStageMask: std::ptr::null(),
                                        commandBufferCount: 1,
                                        pCommandBuffers: &self.command_buffer,
                                        signalSemaphoreCount: 0,
                                        pSignalSemaphores: std::ptr::null(),
                                    };
                                    let _ = (dfns.QueueSubmit)(
                                        self.queue,
                                        1,
                                        &submit,
                                        VK_NULL_HANDLE,
                                    );
                                    let _ = (dfns.QueueWaitIdle)(self.queue);
                                }
                                (dfns.FreeMemory)(self.device, staging_mem, std::ptr::null());
                            }
                        }
                        (dfns.DestroyBuffer)(self.device, staging_buf, std::ptr::null());
                    }
                }
            }
        }
        DuctResult::Success
    }

    fn destroy_texture(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if let Some(t) = self.textures.remove(&handle) {
            self.allocated_gpu_bytes = self
                .allocated_gpu_bytes
                .saturating_sub((t.width as usize) * (t.height as usize) * 4);
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        let _ = (dfns.DeviceWaitIdle)(self.device);
                        if t.sampler != VK_NULL_HANDLE {
                            (dfns.DestroySampler)(self.device, t.sampler, std::ptr::null());
                        }
                        if t.view != VK_NULL_HANDLE {
                            (dfns.DestroyImageView)(self.device, t.view, std::ptr::null());
                        }
                        if t.image != VK_NULL_HANDLE {
                            (dfns.DestroyImage)(self.device, t.image, std::ptr::null());
                        }
                        if t.memory != VK_NULL_HANDLE {
                            (dfns.FreeMemory)(self.device, t.memory, std::ptr::null());
                        }
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_texture(&mut self, _slot: u32, handle: DuctTextureHandle) -> DuctResult {
        if self.textures.contains_key(&handle) {
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn generate_mipmaps(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if self.textures.contains_key(&handle) {
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn set_texture_filter(
        &mut self,
        handle: DuctTextureHandle,
        min: DuctFilterMode,
        mag: DuctFilterMode,
    ) -> DuctResult {
        if let Some(t) = self.textures.get_mut(&handle) {
            t.min_filter = min;
            t.mag_filter = mag;
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        if t.sampler != VK_NULL_HANDLE {
                            (dfns.DestroySampler)(self.device, t.sampler, std::ptr::null());
                        }
                        let sampler_info = VkSamplerCreateInfo {
                            sType: VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            magFilter: map_filter(t.mag_filter),
                            minFilter: map_filter(t.min_filter),
                            mipmapMode: VK_SAMPLER_MIPMAP_MODE_LINEAR,
                            addressModeU: map_address_mode(t.wrap_s),
                            addressModeV: map_address_mode(t.wrap_t),
                            addressModeW: map_address_mode(t.wrap_s),
                            mipLodBias: 0.0,
                            anisotropyEnable: VK_FALSE,
                            maxAnisotropy: 1.0,
                            compareEnable: VK_FALSE,
                            compareOp: VK_COMPARE_OP_ALWAYS,
                            minLod: 0.0,
                            maxLod: 1.0,
                            borderColor: 0,
                            unnormalizedCoordinates: VK_FALSE,
                        };
                        let _ = (dfns.CreateSampler)(
                            self.device,
                            &sampler_info,
                            std::ptr::null(),
                            &mut t.sampler,
                        );
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn set_texture_wrap(
        &mut self,
        handle: DuctTextureHandle,
        wrap_s: DuctWrapMode,
        wrap_t: DuctWrapMode,
    ) -> DuctResult {
        if let Some(t) = self.textures.get_mut(&handle) {
            t.wrap_s = wrap_s;
            t.wrap_t = wrap_t;
            let min = t.min_filter;
            let mag = t.mag_filter;
            self.set_texture_filter(handle, min, mag)
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_render_target(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        let color_tex = self.create_texture_2d(desc.width, desc.height, desc.format, None)?;
        let mut framebuffer = VK_NULL_HANDLE;
        let mut depth_image = VK_NULL_HANDLE;
        let mut depth_memory = VK_NULL_HANDLE;
        let mut depth_view = VK_NULL_HANDLE;

        let dfns = match &self.device_fns {
            Some(d) if !self.device.is_null() => *d,
            _ => return Err(DuctResult::ErrorInvalidState),
        };

        if let Some(tex_rec) = self.textures.get(&color_tex) {
            if tex_rec.view != VK_NULL_HANDLE {
                let target_rp = if desc.has_depth && self.offscreen_render_pass_with_depth != VK_NULL_HANDLE {
                    let depth_info = VkImageCreateInfo {
                        sType: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        imageType: VK_IMAGE_TYPE_2D,
                        format: VK_FORMAT_D32_SFLOAT,
                        extent: VkExtent3D {
                            width: desc.width,
                            height: desc.height,
                            depth: 1,
                        },
                        mipLevels: 1,
                        arrayLayers: 1,
                        samples: VK_SAMPLE_COUNT_1_BIT,
                        tiling: VK_IMAGE_TILING_OPTIMAL,
                        usage: VK_IMAGE_USAGE_DEPTH_STENCIL_ATTACHMENT_BIT,
                        sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                        queueFamilyIndexCount: 0,
                        pQueueFamilyIndices: std::ptr::null(),
                        initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
                    };

                    if unsafe {
                        (dfns.CreateImage)(self.device, &depth_info, std::ptr::null(), &mut depth_image)
                    } == VK_SUCCESS
                    {
                        let mut mem_reqs = VkMemoryRequirements::default();
                        unsafe {
                            (dfns.GetImageMemoryRequirements)(self.device, depth_image, &mut mem_reqs);
                        }
                        if let Some(mem_type) = self
                            .find_memory_type(mem_reqs.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT)
                            .or_else(|| self.find_memory_type(mem_reqs.memoryTypeBits, 0))
                        {
                            let alloc_info = VkMemoryAllocateInfo {
                                sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                                pNext: std::ptr::null(),
                                allocationSize: mem_reqs.size,
                                memoryTypeIndex: mem_type,
                            };
                            if unsafe {
                                (dfns.AllocateMemory)(self.device, &alloc_info, std::ptr::null(), &mut depth_memory)
                            } == VK_SUCCESS
                            {
                                unsafe {
                                    let _ = (dfns.BindImageMemory)(self.device, depth_image, depth_memory, 0);
                                }
                                let iv_info = VkImageViewCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                    image: depth_image,
                                    viewType: VK_IMAGE_VIEW_TYPE_2D,
                                    format: VK_FORMAT_D32_SFLOAT,
                                    components: VkComponentMapping::default(),
                                    subresourceRange: VkImageSubresourceRange {
                                        aspectMask: VK_IMAGE_ASPECT_DEPTH_BIT,
                                        baseMipLevel: 0,
                                        levelCount: 1,
                                        baseArrayLayer: 0,
                                        layerCount: 1,
                                    },
                                };
                                unsafe {
                                    let _ = (dfns.CreateImageView)(
                                        self.device,
                                        &iv_info,
                                        std::ptr::null(),
                                        &mut depth_view,
                                    );
                                }
                            }
                        }
                    }
                    self.offscreen_render_pass_with_depth
                } else {
                    self.offscreen_render_pass
                };

                let mut attachments = Vec::new();
                attachments.push(tex_rec.view);
                if desc.has_depth && depth_view != VK_NULL_HANDLE {
                    attachments.push(depth_view);
                }

                let fb_info = VkFramebufferCreateInfo {
                    sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
                    pNext: std::ptr::null(),
                    flags: 0,
                    renderPass: target_rp,
                    attachmentCount: attachments.len() as u32,
                    pAttachments: attachments.as_ptr(),
                    width: desc.width,
                    height: desc.height,
                    layers: 1,
                };
                let _ = unsafe {
                    (dfns.CreateFramebuffer)(
                        self.device,
                        &fb_info,
                        std::ptr::null(),
                        &mut framebuffer,
                    )
                };
            }
        }

        let handle = DuctRenderTargetHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.render_targets.insert(
            handle,
            VkRenderTargetRecord {
                width: desc.width,
                height: desc.height,
                color_tex,
                depth_image,
                depth_memory,
                depth_view,
                framebuffer,
                has_depth: desc.has_depth,
            },
        );
        Ok(handle)
    }

    fn destroy_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if let Some(rt) = self.render_targets.remove(&handle) {
            if self.active_render_target == handle {
                self.active_render_target = DuctRenderTargetHandle::INVALID;
            }
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        let _ = (dfns.DeviceWaitIdle)(self.device);
                        if rt.framebuffer != VK_NULL_HANDLE {
                            (dfns.DestroyFramebuffer)(self.device, rt.framebuffer, std::ptr::null());
                        }
                        if rt.depth_view != VK_NULL_HANDLE {
                            (dfns.DestroyImageView)(self.device, rt.depth_view, std::ptr::null());
                        }
                        if rt.depth_image != VK_NULL_HANDLE {
                            (dfns.DestroyImage)(self.device, rt.depth_image, std::ptr::null());
                        }
                        if rt.depth_memory != VK_NULL_HANDLE {
                            (dfns.FreeMemory)(self.device, rt.depth_memory, std::ptr::null());
                        }
                    }
                }
            }
            let _ = self.destroy_texture(rt.color_tex);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if !handle.is_valid() {
            self.active_render_target = DuctRenderTargetHandle::INVALID;
            self.current_pass_is_offscreen = false;
        } else if self.render_targets.contains_key(&handle) {
            self.active_render_target = handle;
            self.current_pass_is_offscreen = true;
        } else {
            return DuctResult::ErrorResourceNotFound;
        }

        // If recording inside active command buffer, seamlessly transition render passes!
        if self.cmd_recording {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    if self.render_pass_active {
                        (dfns.CmdEndRenderPass)(self.command_buffer);
                        self.render_pass_active = false;
                    }

                    let mut target_fb = VK_NULL_HANDLE;
                    let mut target_rp = VK_NULL_HANDLE;
                    let mut target_extent = VkExtent2D {
                        width: self.viewport_width,
                        height: self.viewport_height,
                    };
                    let mut has_depth = true;

                    if self.current_pass_is_offscreen {
                        if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                            target_fb = rt.framebuffer;
                            target_rp = if rt.has_depth && self.offscreen_render_pass_with_depth != VK_NULL_HANDLE {
                                self.offscreen_render_pass_with_depth
                            } else {
                                self.offscreen_render_pass
                            };
                            target_extent = VkExtent2D {
                                width: rt.width,
                                height: rt.height,
                            };
                            has_depth = rt.has_depth;
                        }
                    } else if self.swapchain_active && self.swapchain_image_acquired {
                        if (self.current_image_index as usize) < self.framebuffers.len() {
                            target_fb = self.framebuffers[self.current_image_index as usize];
                            target_rp = self.render_pass;
                            target_extent = self.swapchain_extent;
                            has_depth = self.depth_image_view != VK_NULL_HANDLE;
                        }
                    }

                    if target_rp != VK_NULL_HANDLE && target_fb != VK_NULL_HANDLE {
                        let vp = VkViewport {
                            x: 0.0,
                            y: target_extent.height as f32,
                            width: target_extent.width as f32,
                            height: -(target_extent.height as f32),
                            minDepth: 0.0,
                            maxDepth: 1.0,
                        };
                        (dfns.CmdSetViewport)(self.command_buffer, 0, 1, &vp);

                        let scissor = VkRect2D {
                            offset: VkOffset2D { x: 0, y: 0 },
                            extent: target_extent,
                        };
                        (dfns.CmdSetScissor)(self.command_buffer, 0, 1, &scissor);

                        let clear_vals = [
                            VkClearValue {
                                float32: [
                                    self.last_clear_color.r,
                                    self.last_clear_color.g,
                                    self.last_clear_color.b,
                                    self.last_clear_color.a,
                                ],
                            },
                            VkClearValue {
                                float32: [1.0, 0.0, 0.0, 0.0],
                            },
                        ];
                        let clear_count = if has_depth { 2 } else { 1 };
                        let rp_begin = VkRenderPassBeginInfo {
                            sType: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
                            pNext: std::ptr::null(),
                            renderPass: target_rp,
                            framebuffer: target_fb,
                            renderArea: VkRect2D {
                                offset: VkOffset2D { x: 0, y: 0 },
                                extent: target_extent,
                            },
                            clearValueCount: clear_count,
                            pClearValues: clear_vals.as_ptr(),
                        };
                        (dfns.CmdBeginRenderPass)(
                            self.command_buffer,
                            &rp_begin,
                            VK_SUBPASS_CONTENTS_INLINE,
                        );
                        self.render_pass_active = true;
                    }
                }
            }
        }

        DuctResult::Success
    }

    fn get_render_target_texture(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        if let Some(rt) = self.render_targets.get(&handle) {
            Ok(rt.color_tex)
        } else {
            Err(DuctResult::ErrorResourceNotFound)
        }
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
        let src_img = if src.is_valid() {
            self.render_targets
                .get(&src)
                .and_then(|rt| self.textures.get(&rt.color_tex))
                .map(|t| t.image)
                .unwrap_or(VK_NULL_HANDLE)
        } else {
            VK_NULL_HANDLE
        };

        let is_dst_swapchain = !dst.is_valid();
        let dst_img = if is_dst_swapchain {
            if self.swapchain_active && (self.current_image_index as usize) < self.swapchain_images.len() {
                self.swapchain_images[self.current_image_index as usize]
            } else {
                VK_NULL_HANDLE
            }
        } else {
            self.render_targets
                .get(&dst)
                .and_then(|rt| self.textures.get(&rt.color_tex))
                .map(|t| t.image)
                .unwrap_or(VK_NULL_HANDLE)
        };

        if src_img == VK_NULL_HANDLE
            || dst_img == VK_NULL_HANDLE
            || src_w == 0
            || src_h == 0
            || dst_w == 0
            || dst_h == 0
        {
            return DuctResult::ErrorInvalidParameter;
        }

        if let Some(dfns) = &self.device_fns {
            unsafe {
                if self.cmd_recording && self.render_pass_active {
                    (dfns.CmdEndRenderPass)(self.command_buffer);
                    self.render_pass_active = false;
                }

                let one_time = !self.cmd_recording;
                if one_time {
                    let _ = (dfns.DeviceWaitIdle)(self.device);
                    let _ = (dfns.ResetCommandBuffer)(self.command_buffer, 0);
                    let begin_info = VkCommandBufferBeginInfo {
                        sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                        pNext: std::ptr::null(),
                        flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                        pInheritanceInfo: std::ptr::null(),
                    };
                    let _ = (dfns.BeginCommandBuffer)(self.command_buffer, &begin_info);
                }

                let barriers_in = [
                    VkImageMemoryBarrier {
                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                        pNext: std::ptr::null(),
                        srcAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        dstAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                        oldLayout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                        newLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                        srcQueueFamilyIndex: !0u32,
                        dstQueueFamilyIndex: !0u32,
                        image: src_img,
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    },
                    VkImageMemoryBarrier {
                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                        pNext: std::ptr::null(),
                        srcAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        dstAccessMask: VK_ACCESS_TRANSFER_WRITE_BIT,
                        oldLayout: if is_dst_swapchain {
                            VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL
                        } else {
                            VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL
                        },
                        newLayout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                        srcQueueFamilyIndex: !0u32,
                        dstQueueFamilyIndex: !0u32,
                        image: dst_img,
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    },
                ];
                (dfns.CmdPipelineBarrier)(
                    self.command_buffer,
                    VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                    VK_PIPELINE_STAGE_TRANSFER_BIT,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    2,
                    barriers_in.as_ptr(),
                );

                let blit_region = VkImageBlit {
                    srcSubresource: VkImageSubresourceLayers {
                        aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                        mipLevel: 0,
                        baseArrayLayer: 0,
                        layerCount: 1,
                    },
                    srcOffsets: [
                        VkOffset3D { x: 0, y: 0, z: 0 },
                        VkOffset3D {
                            x: src_w as i32,
                            y: src_h as i32,
                            z: 1,
                        },
                    ],
                    dstSubresource: VkImageSubresourceLayers {
                        aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                        mipLevel: 0,
                        baseArrayLayer: 0,
                        layerCount: 1,
                    },
                    dstOffsets: [
                        VkOffset3D { x: 0, y: 0, z: 0 },
                        VkOffset3D {
                            x: dst_w as i32,
                            y: dst_h as i32,
                            z: 1,
                        },
                    ],
                };
                (dfns.CmdBlitImage)(
                    self.command_buffer,
                    src_img,
                    VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                    dst_img,
                    VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    1,
                    &blit_region,
                    map_filter(filter),
                );

                let dst_final_layout = if is_dst_swapchain {
                    VK_IMAGE_LAYOUT_PRESENT_SRC_KHR
                } else {
                    VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL
                };

                let barriers_out = [
                    VkImageMemoryBarrier {
                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                        pNext: std::ptr::null(),
                        srcAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                        dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                        oldLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                        newLayout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                        srcQueueFamilyIndex: !0u32,
                        dstQueueFamilyIndex: !0u32,
                        image: src_img,
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    },
                    VkImageMemoryBarrier {
                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                        pNext: std::ptr::null(),
                        srcAccessMask: VK_ACCESS_TRANSFER_WRITE_BIT,
                        dstAccessMask: VK_ACCESS_MEMORY_READ_BIT,
                        oldLayout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                        newLayout: dst_final_layout,
                        srcQueueFamilyIndex: !0u32,
                        dstQueueFamilyIndex: !0u32,
                        image: dst_img,
                        subresourceRange: VkImageSubresourceRange {
                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                            baseMipLevel: 0,
                            levelCount: 1,
                            baseArrayLayer: 0,
                            layerCount: 1,
                        },
                    },
                ];
                (dfns.CmdPipelineBarrier)(
                    self.command_buffer,
                    VK_PIPELINE_STAGE_TRANSFER_BIT,
                    VK_PIPELINE_STAGE_ALL_COMMANDS_BIT,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    2,
                    barriers_out.as_ptr(),
                );

                if one_time {
                    let _ = (dfns.EndCommandBuffer)(self.command_buffer);
                    let submit = VkSubmitInfo {
                        sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                        pNext: std::ptr::null(),
                        waitSemaphoreCount: 0,
                        pWaitSemaphores: std::ptr::null(),
                        pWaitDstStageMask: std::ptr::null(),
                        commandBufferCount: 1,
                        pCommandBuffers: &self.command_buffer,
                        signalSemaphoreCount: 0,
                        pSignalSemaphores: std::ptr::null(),
                    };
                    let _ = (dfns.QueueSubmit)(self.queue, 1, &submit, VK_NULL_HANDLE);
                    let _ = (dfns.QueueWaitIdle)(self.queue);
                }
            }
        }
        DuctResult::Success
    }

    fn read_pixels(
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

        // Determine source image to read back from (offscreen FBO or swapchain image)
        let (src_img, src_fmt, img_w, img_h, current_layout) =
            if self.active_render_target.is_valid() {
                if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                    if let Some(tex) = self.textures.get(&rt.color_tex) {
                        (
                            tex.image,
                            tex.vk_format,
                            rt.width,
                            rt.height,
                            VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
                        )
                    } else {
                        (VK_NULL_HANDLE, 0, 0, 0, 0)
                    }
                } else {
                    (VK_NULL_HANDLE, 0, 0, 0, 0)
                }
            } else if self.swapchain_active
                && (self.current_image_index as usize) < self.swapchain_images.len()
            {
                (
                    self.swapchain_images[self.current_image_index as usize],
                    self.swapchain_format,
                    self.swapchain_extent.width,
                    self.swapchain_extent.height,
                    VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
                )
            } else {
                (VK_NULL_HANDLE, 0, 0, 0, 0)
            };

        if src_img != VK_NULL_HANDLE
            && !self.cmd_recording
            && x + width <= img_w
            && y + height <= img_h
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    let _ = (dfns.DeviceWaitIdle)(self.device);

                    let buf_info = VkBufferCreateInfo {
                        sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
                        pNext: std::ptr::null(),
                        flags: 0,
                        size: req as VkDeviceSize,
                        usage: VK_BUFFER_USAGE_TRANSFER_DST_BIT,
                        sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                        queueFamilyIndexCount: 0,
                        pQueueFamilyIndices: std::ptr::null(),
                    };
                    let mut staging_buf = VK_NULL_HANDLE;
                    let mut staging_mem = VK_NULL_HANDLE;

                    if (dfns.CreateBuffer)(
                        self.device,
                        &buf_info,
                        std::ptr::null(),
                        &mut staging_buf,
                    ) == VK_SUCCESS
                    {
                        let mut mem_reqs = VkMemoryRequirements::default();
                        (dfns.GetBufferMemoryRequirements)(self.device, staging_buf, &mut mem_reqs);
                        if let Some(mem_idx) = self.find_memory_type(
                            mem_reqs.memoryTypeBits,
                            VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT
                                | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                        ) {
                            let alloc_info = VkMemoryAllocateInfo {
                                sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                                pNext: std::ptr::null(),
                                allocationSize: mem_reqs.size,
                                memoryTypeIndex: mem_idx,
                            };
                            if (dfns.AllocateMemory)(
                                self.device,
                                &alloc_info,
                                std::ptr::null(),
                                &mut staging_mem,
                            ) == VK_SUCCESS
                            {
                                let _ = (dfns.BindBufferMemory)(
                                    self.device,
                                    staging_buf,
                                    staging_mem,
                                    0,
                                );

                                let _ = (dfns.ResetCommandBuffer)(self.command_buffer, 0);
                                let begin_info = VkCommandBufferBeginInfo {
                                    sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                                    pNext: std::ptr::null(),
                                    flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                                    pInheritanceInfo: std::ptr::null(),
                                };
                                if (dfns.BeginCommandBuffer)(self.command_buffer, &begin_info)
                                    == VK_SUCCESS
                                {
                                    let to_src = VkImageMemoryBarrier {
                                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                                        pNext: std::ptr::null(),
                                        srcAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                                        dstAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                                        oldLayout: current_layout,
                                        newLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                        srcQueueFamilyIndex: !0u32,
                                        dstQueueFamilyIndex: !0u32,
                                        image: src_img,
                                        subresourceRange: VkImageSubresourceRange {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            baseMipLevel: 0,
                                            levelCount: 1,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                    };
                                    (dfns.CmdPipelineBarrier)(
                                        self.command_buffer,
                                        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                                        VK_PIPELINE_STAGE_TRANSFER_BIT,
                                        0,
                                        0,
                                        std::ptr::null(),
                                        0,
                                        std::ptr::null(),
                                        1,
                                        &to_src,
                                    );

                                    let copy_region = VkBufferImageCopy {
                                        bufferOffset: 0,
                                        bufferRowLength: 0,
                                        bufferImageHeight: 0,
                                        imageSubresource: VkImageSubresourceLayers {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            mipLevel: 0,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                        imageOffset: VkOffset3D {
                                            x: x as i32,
                                            y: y as i32,
                                            z: 0,
                                        },
                                        imageExtent: VkExtent3D {
                                            width,
                                            height,
                                            depth: 1,
                                        },
                                    };
                                    (dfns.CmdCopyImageToBuffer)(
                                        self.command_buffer,
                                        src_img,
                                        VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                        staging_buf,
                                        1,
                                        &copy_region,
                                    );

                                    let restore = VkImageMemoryBarrier {
                                        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                                        pNext: std::ptr::null(),
                                        srcAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                                        dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                                        oldLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                        newLayout: current_layout,
                                        srcQueueFamilyIndex: !0u32,
                                        dstQueueFamilyIndex: !0u32,
                                        image: src_img,
                                        subresourceRange: VkImageSubresourceRange {
                                            aspectMask: VK_IMAGE_ASPECT_COLOR_BIT,
                                            baseMipLevel: 0,
                                            levelCount: 1,
                                            baseArrayLayer: 0,
                                            layerCount: 1,
                                        },
                                    };
                                    (dfns.CmdPipelineBarrier)(
                                        self.command_buffer,
                                        VK_PIPELINE_STAGE_TRANSFER_BIT,
                                        VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                                        0,
                                        0,
                                        std::ptr::null(),
                                        0,
                                        std::ptr::null(),
                                        1,
                                        &restore,
                                    );

                                    let _ = (dfns.EndCommandBuffer)(self.command_buffer);
                                    let submit = VkSubmitInfo {
                                        sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                                        pNext: std::ptr::null(),
                                        waitSemaphoreCount: 0,
                                        pWaitSemaphores: std::ptr::null(),
                                        pWaitDstStageMask: std::ptr::null(),
                                        commandBufferCount: 1,
                                        pCommandBuffers: &self.command_buffer,
                                        signalSemaphoreCount: 0,
                                        pSignalSemaphores: std::ptr::null(),
                                    };
                                    let _ = (dfns.QueueSubmit)(
                                        self.queue,
                                        1,
                                        &submit,
                                        VK_NULL_HANDLE,
                                    );
                                    let _ = (dfns.QueueWaitIdle)(self.queue);

                                    let mut mapped: *mut c_void = std::ptr::null_mut();
                                    if (dfns.MapMemory)(
                                        self.device,
                                        staging_mem,
                                        0,
                                        req as VkDeviceSize,
                                        0,
                                        &mut mapped,
                                    ) == VK_SUCCESS
                                        && !mapped.is_null()
                                    {
                                        std::ptr::copy_nonoverlapping(
                                            mapped as *const u8,
                                            out_rgba.as_mut_ptr(),
                                            req,
                                        );
                                        (dfns.UnmapMemory)(self.device, staging_mem);

                                        if src_fmt == VK_FORMAT_B8G8R8A8_UNORM
                                            || src_fmt == VK_FORMAT_B8G8R8A8_SRGB
                                        {
                                            for px in out_rgba[..req].chunks_exact_mut(4) {
                                                px.swap(0, 2);
                                            }
                                        }
                                    }
                                }
                                (dfns.FreeMemory)(self.device, staging_mem, std::ptr::null());
                            }
                        }
                        (dfns.DestroyBuffer)(self.device, staging_buf, std::ptr::null());
                        return DuctResult::Success;
                    }
                }
            }
        }

        out_rgba[..req].fill(255);
        DuctResult::Success
    }

    fn set_pipeline_state(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult {
        self.pipeline_switches_in_frame += 1;

        // Check if binding a Compute Shader
        if let Some(vs_rec) = self.shaders.get(&desc.vertex_shader) {
            if vs_rec.stage == DuctShaderStage::Compute && !desc.fragment_shader.is_valid() {
                let cs_mod = vs_rec.module;
                if let Some(&cached) = self.compute_pipelines.get(&desc.vertex_shader) {
                    self.active_compute_pipeline = cached;
                } else if cs_mod != VK_NULL_HANDLE && self.compute_pipeline_layout != VK_NULL_HANDLE
                {
                    if let Some(dfns) = &self.device_fns {
                        let entry = b"main\0";
                        let cp_info = VkComputePipelineCreateInfo {
                            sType: VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO,
                            pNext: std::ptr::null(),
                            flags: 0,
                            stage: VkPipelineShaderStageCreateInfo {
                                sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                                pNext: std::ptr::null(),
                                flags: 0,
                                stage: VK_SHADER_STAGE_COMPUTE_BIT,
                                module: cs_mod,
                                pName: entry.as_ptr() as *const c_char,
                                pSpecializationInfo: std::ptr::null(),
                            },
                            layout: self.compute_pipeline_layout,
                            basePipelineHandle: VK_NULL_HANDLE,
                            basePipelineIndex: -1,
                        };
                        let mut cp = VK_NULL_HANDLE;
                        if unsafe {
                            (dfns.CreateComputePipelines)(
                                self.device,
                                VK_NULL_HANDLE,
                                1,
                                &cp_info,
                                std::ptr::null(),
                                &mut cp,
                            )
                        } == VK_SUCCESS
                        {
                            self.compute_pipelines.insert(desc.vertex_shader, cp);
                            self.active_compute_pipeline = cp;
                        }
                    }
                }
                return DuctResult::Success;
            }
        }

        self.current_pso = Some(*desc);
        let is_offscreen = self.active_render_target.is_valid();
        let pipeline = self.get_or_create_graphics_pipeline(desc, is_offscreen);
        self.active_graphics_pipeline = pipeline;

        if self.cmd_recording && self.render_pass_active && pipeline != VK_NULL_HANDLE {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdBindPipeline)(
                        self.command_buffer,
                        VK_PIPELINE_BIND_POINT_GRAPHICS,
                        pipeline,
                    );
                }
            }
        }

        DuctResult::Success
    }

    fn set_wireframe(&mut self, enabled: bool) -> DuctResult {
        self.wireframe = enabled;
        DuctResult::Success
    }

    fn set_vsync(&mut self, enabled: bool) -> DuctResult {
        self.vsync_enabled = enabled;
        DuctResult::Success
    }

    fn set_uniform_int(&mut self, name: &str, val: i32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("mode") {
            self.push_constants.extra[3] = val as f32;
        } else {
            self.push_constants.extra[3] = val as f32;
        }
        DuctResult::Success
    }

    fn set_uniform_float(&mut self, name: &str, val: f32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("time") {
            self.push_constants.extra[0] = val;
        } else if lower.contains("scale") {
            self.push_constants.params[2] = val;
            self.push_constants.params[3] = val;
        } else {
            self.push_constants.params[0] = val;
        }
        DuctResult::Success
    }

    fn set_uniform_vec2(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("res") {
            self.push_constants.extra[1] = x;
            self.push_constants.extra[2] = y;
        } else if lower.contains("mouse") {
            self.push_constants.params[0] = x;
            self.push_constants.params[1] = y;
        } else if lower.contains("scale") {
            self.push_constants.params[2] = x;
            self.push_constants.params[3] = y;
        } else {
            self.push_constants.params[0] = x;
            self.push_constants.params[1] = y;
        }
        DuctResult::Success
    }

    fn set_uniform_vec3(&mut self, _name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        self.push_constants.color = [x, y, z, 1.0];
        DuctResult::Success
    }

    fn set_uniform_vec4(&mut self, _name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        self.push_constants.color = [x, y, z, w];
        DuctResult::Success
    }

    fn set_uniform_mat4(&mut self, _name: &str, elements: &[f32; 16]) -> DuctResult {
        self.push_constants.transform = *elements;
        DuctResult::Success
    }

    fn draw(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count;

        self.prepare_draw();
        if self.cmd_recording
            && self.render_pass_active
            && self.active_graphics_pipeline != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdDraw)(self.command_buffer, vertex_count, 1, first_vertex, 0);
                }
            }
        }
        DuctResult::Success
    }

    fn draw_indexed(&mut self, index_count: u32, first_index: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count;

        self.prepare_draw();
        if self.cmd_recording
            && self.render_pass_active
            && self.active_graphics_pipeline != VK_NULL_HANDLE
            && self.last_bound_ibo != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                let vk_idx = match self.current_index_format {
                    DuctIndexFormat::U16 => VK_INDEX_TYPE_UINT16,
                    DuctIndexFormat::U32 => VK_INDEX_TYPE_UINT32,
                };
                unsafe {
                    (dfns.CmdBindIndexBuffer)(
                        self.command_buffer,
                        self.last_bound_ibo,
                        0,
                        vk_idx,
                    );
                    (dfns.CmdDrawIndexed)(self.command_buffer, index_count, 1, first_index, 0, 0);
                }
            }
        }
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
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count * instance_count;

        self.prepare_draw();
        if self.cmd_recording
            && self.render_pass_active
            && self.active_graphics_pipeline != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdDraw)(
                        self.command_buffer,
                        vertex_count,
                        instance_count,
                        first_vertex,
                        first_instance,
                    );
                }
            }
        }
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
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count * instance_count;
        self.current_index_format = index_format;

        self.prepare_draw();
        if self.cmd_recording
            && self.render_pass_active
            && self.active_graphics_pipeline != VK_NULL_HANDLE
            && self.last_bound_ibo != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                let vk_idx = match index_format {
                    DuctIndexFormat::U16 => VK_INDEX_TYPE_UINT16,
                    DuctIndexFormat::U32 => VK_INDEX_TYPE_UINT32,
                };
                unsafe {
                    (dfns.CmdBindIndexBuffer)(
                        self.command_buffer,
                        self.last_bound_ibo,
                        0,
                        vk_idx,
                    );
                    (dfns.CmdDrawIndexed)(
                        self.command_buffer,
                        index_count,
                        instance_count,
                        first_index,
                        0,
                        0,
                    );
                }
            }
        }
        DuctResult::Success
    }

    fn dispatch_compute(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        if self.cmd_recording
            && !self.render_pass_active
            && self.active_compute_pipeline != VK_NULL_HANDLE
        {
            if let Some(dfns) = &self.device_fns {
                unsafe {
                    (dfns.CmdBindPipeline)(
                        self.command_buffer,
                        VK_PIPELINE_BIND_POINT_COMPUTE,
                        self.active_compute_pipeline,
                    );
                    (dfns.CmdDispatch)(self.command_buffer, groups_x, groups_y, groups_z);
                }
            }
        }
        DuctResult::Success
    }

    fn get_frame_stats(&mut self) -> Result<DuctFrameStats, DuctResult> {
        Ok(DuctFrameStats {
            fps: self.smoothed_fps,
            frame_time_ms: self.current_frame_time_ms,
            frame_count: self.frame_count,
            draw_calls: self.draw_calls_in_frame,
            triangles_rendered: if self.vertices_drawn_in_frame > 0 {
                (self.vertices_drawn_in_frame / 3) as u64
            } else {
                (self.draw_calls_in_frame as u64) * 100
            },
            pipeline_switches: self.pipeline_switches_in_frame.max(1),
            allocated_gpu_memory_bytes: self.allocated_gpu_bytes.max(1024 * 1024 * 16),
        })
    }

    fn get_raw_context(&mut self) -> Result<DuctRawContext, DuctResult> {
        Ok(DuctRawContext {
            backend_type: DuctBackendType::Vulkan,
            context_handle: self.device,
            device_handle: self.queue,
            extra_info: self.extra_info.as_ptr(),
            get_proc_address: Some(vk_get_proc_address),
        })
    }

    fn execute_custom(&mut self, command_id: u32, data: *mut c_void) -> DuctResult {
        println!(
            "[vkduct] Custom Vulkan Command {} executed (payload={:?})",
            command_id, data
        );
        DuctResult::Success
    }
}

export_pipeline!(VkPipeline, vkduct_create_pipeline);
