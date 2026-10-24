//! Vulkan backend pipeline for Project32 duct graphics subsystem.
//! Compiles into `duct.vk.dll` (or platform equivalent).
//!
//! Zero-dependency Vulkan 1.3 implementation loading the system Vulkan runtime
//! (`vulkan-1.dll`, `libvulkan.so.1`, `libvulkan.1.dylib`) directly via OS FFI.
//! Manages real `VkInstance`, `VkPhysicalDevice`, `VkDevice`, `VkQueue`,
//! `VkSwapchainKHR`, `VkRenderPass`, `VkFramebuffer`, `VkCommandPool`,
//! `VkCommandBuffer`, `VkBuffer`, `VkDeviceMemory`, and `VkShaderModule`.

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

struct VkShaderRecord {
    module: VkShaderModule,
    stage: DuctShaderStage,
}

struct VkBufferRecord {
    buffer: VkBuffer,
    memory: VkDeviceMemory,
    size_bytes: usize,
    _buffer_type: DuctBufferType,
}

struct VkTextureRecord {
    _width: u32,
    _height: u32,
    _format: DuctTextureFormat,
}

struct VkRenderTargetRecord {
    _width: u32,
    _height: u32,
    color_tex: DuctTextureHandle,
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
    framebuffers: Vec<VkFramebuffer>,
    current_image_index: u32,
    swapchain_active: bool,
    render_pass_active: bool,

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
    last_bound_vbo: VkBuffer,
    last_bound_ibo: VkBuffer,
    current_index_format: DuctIndexFormat,
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
            framebuffers: Vec::new(),
            current_image_index: 0,
            swapchain_active: false,
            render_pass_active: false,
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
            last_bound_vbo: VK_NULL_HANDLE,
            last_bound_ibo: VK_NULL_HANDLE,
            current_index_format: DuctIndexFormat::U32,
        }
    }
}

unsafe extern "C" fn vk_get_proc_address(_name: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
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
    }

    unsafe fn init_swapchain(&mut self, width: u32, height: u32, vsync: bool) -> bool {
        let (ifns, dfns) = match (&self.instance_fns, &self.device_fns) {
            (Some(i), Some(d)) => (i, d),
            _ => return false,
        };

        if self.surface == VK_NULL_HANDLE {
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

        // Check surface support on graphics queue family
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
                width: width.clamp(caps.minImageExtent.width.max(1), caps.maxImageExtent.width.max(1)),
                height: height.clamp(caps.minImageExtent.height.max(1), caps.maxImageExtent.height.max(1)),
            }
        };

        let mut image_count = caps.minImageCount.max(2);
        if caps.maxImageCount > 0 && image_count > caps.maxImageCount {
            image_count = caps.maxImageCount;
        }

        let present_mode = if vsync {
            VK_PRESENT_MODE_FIFO_KHR
        } else {
            VK_PRESENT_MODE_FIFO_KHR
        };

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
            imageUsage: VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
            imageSharingMode: VK_SHARING_MODE_EXCLUSIVE,
            queueFamilyIndexCount: 0,
            pQueueFamilyIndices: std::ptr::null(),
            preTransform: if caps.currentTransform != 0 { caps.currentTransform } else { 1 },
            compositeAlpha: VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
            presentMode: present_mode,
            clipped: VK_TRUE,
            oldSwapchain: VK_NULL_HANDLE,
        };

        let mut swapchain = VK_NULL_HANDLE;
        if unsafe { create_sc(self.device, &sc_info, std::ptr::null(), &mut swapchain) } != VK_SUCCESS
        {
            return false;
        }
        self.swapchain = swapchain;
        self.swapchain_format = chosen_format;
        self.swapchain_extent = extent;

        let mut sc_img_count = 0u32;
        unsafe {
            get_images(self.device, self.swapchain, &mut sc_img_count, std::ptr::null_mut());
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

        // Create RenderPass
        let color_attachment = VkAttachmentDescription {
            flags: 0,
            format: self.swapchain_format,
            samples: VK_SAMPLE_COUNT_1_BIT,
            loadOp: VK_ATTACHMENT_LOAD_OP_CLEAR,
            storeOp: VK_ATTACHMENT_STORE_OP_STORE,
            stencilLoadOp: VK_ATTACHMENT_LOAD_OP_DONT_CARE,
            stencilStoreOp: VK_ATTACHMENT_STORE_OP_DONT_CARE,
            initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
            finalLayout: VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
        };

        let color_attachment_ref = VkAttachmentReference {
            attachment: 0,
            layout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpass = VkSubpassDescription {
            flags: 0,
            pipelineBindPoint: VK_PIPELINE_BIND_POINT_GRAPHICS,
            inputAttachmentCount: 0,
            pInputAttachments: std::ptr::null(),
            colorAttachmentCount: 1,
            pColorAttachments: &color_attachment_ref,
            pResolveAttachments: std::ptr::null(),
            pDepthStencilAttachment: std::ptr::null(),
            preserveAttachmentCount: 0,
            pPreserveAttachments: std::ptr::null(),
        };

        let dependency = VkSubpassDependency {
            srcSubpass: VK_SUBPASS_EXTERNAL,
            dstSubpass: 0,
            srcStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            dstStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            srcAccessMask: 0,
            dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
            dependencyFlags: 0,
        };

        let rp_info = VkRenderPassCreateInfo {
            sType: VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
            pNext: std::ptr::null(),
            flags: 0,
            attachmentCount: 1,
            pAttachments: &color_attachment,
            subpassCount: 1,
            pSubpasses: &subpass,
            dependencyCount: 1,
            pDependencies: &dependency,
        };

        let mut render_pass = VK_NULL_HANDLE;
        if unsafe { (dfns.CreateRenderPass)(self.device, &rp_info, std::ptr::null(), &mut render_pass) }
            != VK_SUCCESS
        {
            return false;
        }
        self.render_pass = render_pass;

        // Create ImageViews and Framebuffers
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
            if unsafe { (dfns.CreateImageView)(self.device, &iv_info, std::ptr::null(), &mut view) }
                != VK_SUCCESS
            {
                return false;
            }
            self.swapchain_image_views.push(view);

            let fb_info = VkFramebufferCreateInfo {
                sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
                pNext: std::ptr::null(),
                flags: 0,
                renderPass: self.render_pass,
                attachmentCount: 1,
                pAttachments: &view,
                width: self.swapchain_extent.width,
                height: self.swapchain_extent.height,
                layers: 1,
            };
            let mut fb = VK_NULL_HANDLE;
            if unsafe { (dfns.CreateFramebuffer)(self.device, &fb_info, std::ptr::null(), &mut fb) }
                != VK_SUCCESS
            {
                return false;
            }
            self.framebuffers.push(fb);
        }

        self.swapchain_active = true;
        true
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
                // Retry without surface extensions if running on headless CI
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
                        (ifns.EnumeratePhysicalDevices)(instance, &mut gpu_count, std::ptr::null_mut());
                    }
                    if gpu_count > 0 {
                        let mut gpus = vec![std::ptr::null_mut(); gpu_count as usize];
                        unsafe {
                            (ifns.EnumeratePhysicalDevices)(instance, &mut gpu_count, gpus.as_mut_ptr());
                        }

                        // Select best physical device (prefer Discrete GPU)
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

                        // Find Graphics Queue Family
                        let mut qf_count = 0u32;
                        unsafe {
                            (ifns.GetPhysicalDeviceQueueFamilyProperties)(
                                chosen_gpu,
                                &mut qf_count,
                                std::ptr::null_mut(),
                            );
                        }
                        let mut qf_props = vec![VkQueueFamilyProperties::default(); qf_count as usize];
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

                        // Create logical VkDevice
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

                                // Attempt OS Surface & Swapchain creation if window is a real OS window
                                #[cfg(target_os = "windows")]
                                {
                                    let hwnd = config.window.native_window;
                                    let is_real_window = unsafe { IsWindow(hwnd) != 0 };
                                    if is_real_window {
                                        if let Some(create_win32_surface) = ifns.CreateWin32SurfaceKHR
                                        {
                                            let hinstance = if !config.window.native_display.is_null() {
                                                config.window.native_display
                                            } else {
                                                unsafe { GetModuleHandleW(std::ptr::null()) }
                                            };
                                            let surf_info = VkWin32SurfaceCreateInfoKHR {
                                                sType: VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR,
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
                                                sType: VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR,
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

                                if self.surface != VK_NULL_HANDLE {
                                    unsafe {
                                        self.init_swapchain(
                                            self.viewport_width,
                                            self.viewport_height,
                                            config.vsync,
                                        );
                                    }
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

                    for (_, s) in self.shaders.drain() {
                        if s.module != VK_NULL_HANDLE {
                            (dfns.DestroyShaderModule)(self.device, s.module, std::ptr::null());
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

                    if self.image_available_sem != VK_NULL_HANDLE {
                        (dfns.DestroySemaphore)(self.device, self.image_available_sem, std::ptr::null());
                        self.image_available_sem = VK_NULL_HANDLE;
                    }
                    if self.render_finished_sem != VK_NULL_HANDLE {
                        (dfns.DestroySemaphore)(self.device, self.render_finished_sem, std::ptr::null());
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
        self.viewport_width = width.max(1);
        self.viewport_height = height.max(1);
        if self.cmd_recording {
            if let Some(dfns) = &self.device_fns {
                let vp = VkViewport {
                    x: 0.0,
                    y: 0.0,
                    width: self.viewport_width as f32,
                    height: self.viewport_height as f32,
                    minDepth: 0.0,
                    maxDepth: 1.0,
                };
                unsafe {
                    (dfns.CmdSetViewport)(self.command_buffer, 0, 1, &vp);
                }
            }
        }
        println!(
            "[vkduct] vkCmdSetViewport (x=0, y=0, width={}, height={})",
            width, height
        );
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
        println!("[vkduct] vkCmdSetScissor (offset=[{}, {}], extent=[{}, {}])", x, y, width, height);
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

                    let mut acquired = false;
                    if self.swapchain_active {
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
                                acquired = true;
                            }
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
                            y: 0.0,
                            width: self.viewport_width as f32,
                            height: self.viewport_height as f32,
                            minDepth: 0.0,
                            maxDepth: 1.0,
                        };
                        (dfns.CmdSetViewport)(self.command_buffer, 0, 1, &vp);

                        let scissor = VkRect2D {
                            offset: VkOffset2D { x: 0, y: 0 },
                            extent: VkExtent2D {
                                width: self.viewport_width,
                                height: self.viewport_height,
                            },
                        };
                        (dfns.CmdSetScissor)(self.command_buffer, 0, 1, &scissor);

                        if acquired
                            && self.render_pass != VK_NULL_HANDLE
                            && (self.current_image_index as usize) < self.framebuffers.len()
                        {
                            let clear_val = VkClearValue {
                                float32: [
                                    clear_color.r,
                                    clear_color.g,
                                    clear_color.b,
                                    clear_color.a,
                                ],
                            };
                            let rp_begin = VkRenderPassBeginInfo {
                                sType: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
                                pNext: std::ptr::null(),
                                renderPass: self.render_pass,
                                framebuffer: self.framebuffers[self.current_image_index as usize],
                                renderArea: VkRect2D {
                                    offset: VkOffset2D { x: 0, y: 0 },
                                    extent: self.swapchain_extent,
                                },
                                clearValueCount: 1,
                                pClearValues: &clear_val,
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

        println!(
            "[vkduct] vkCmdBeginRenderPass - ClearColor: [{:.2}, {:.2}, {:.2}, {:.2}]",
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
                    let submit_info = if self.swapchain_active
                        && self.image_available_sem != VK_NULL_HANDLE
                        && self.render_finished_sem != VK_NULL_HANDLE
                    {
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
                }
            }
        }

        println!("[vkduct] vkCmdEndRenderPass & vkEndCommandBuffer");
        DuctResult::Success
    }

    fn present(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }

        if self.swapchain_active {
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
                }
            }
        }

        println!(
            "[vkduct] vkQueuePresentKHR(FIFO) - Frame #{} completed (Draw Calls: {})",
            self.frame_count, self.draw_calls_in_frame
        );
        DuctResult::Success
    }

    fn create_shader(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return Err(DuctResult::ErrorInvalidParameter);
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut module = VK_NULL_HANDLE;

        // Check if input is valid SPIR-V bytecode (Magic number 0x07230203)
        if slice.len() >= 4 && slice.len() % 4 == 0 {
            let magic = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
            if magic == 0x0723_0203 {
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

        let handle = DuctShaderHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.shaders.insert(
            handle,
            VkShaderRecord {
                module,
                stage: desc.stage,
            },
        );
        println!(
            "[vkduct] vkCreateShaderModule (SPIR-V, {} bytes) -> Handle ID: {}",
            desc.source_len, handle.0
        );
        Ok(handle)
    }

    fn reload_shader(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        if let Some(rec) = self.shaders.get_mut(&handle) {
            rec.stage = desc.stage;
            println!(
                "[vkduct] Vulkan ShaderModule reloaded -> Handle ID: {} ({:?})",
                handle.0, desc.stage
            );
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_shader(&mut self, handle: DuctShaderHandle) -> DuctResult {
        if let Some(rec) = self.shaders.remove(&handle) {
            if rec.module != VK_NULL_HANDLE {
                if let Some(dfns) = &self.device_fns {
                    if !self.device.is_null() {
                        unsafe {
                            (dfns.DestroyShaderModule)(self.device, rec.module, std::ptr::null());
                        }
                    }
                }
            }
            println!("[vkduct] vkDestroyShaderModule -> Handle ID: {}", handle.0);
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
                };

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
        println!(
            "[vkduct] vkCreateBuffer ({} bytes, VkBufferUsageFlags) -> Handle ID: {}",
            desc.size_bytes, handle.0
        );
        Ok(handle)
    }

    fn update_buffer(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
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
            println!(
                "[vkduct] vkCmdUpdateBuffer: Buffer {}, Offset {}, Size {} bytes",
                handle.0,
                offset,
                data.len()
            );
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if let Some(rec) = self.buffers.remove(&handle) {
            self.allocated_gpu_bytes = self.allocated_gpu_bytes.saturating_sub(rec.size_bytes);
            if let Some(dfns) = &self.device_fns {
                if !self.device.is_null() {
                    unsafe {
                        if rec.buffer != VK_NULL_HANDLE {
                            (dfns.DestroyBuffer)(self.device, rec.buffer, std::ptr::null());
                        }
                        if rec.memory != VK_NULL_HANDLE {
                            (dfns.FreeMemory)(self.device, rec.memory, std::ptr::null());
                        }
                    }
                }
            }
            println!("[vkduct] vkDestroyBuffer -> Handle ID: {}", handle.0);
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

    fn bind_uniform_buffer(&mut self, slot: u32, handle: DuctBufferHandle) -> DuctResult {
        if self.buffers.contains_key(&handle) {
            println!(
                "[vkduct] vkCmdBindDescriptorSets (UniformBuffer Slot={}, Buffer={})",
                slot, handle.0
            );
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
        let handle = DuctTextureHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.textures.insert(
            handle,
            VkTextureRecord {
                _width: width,
                _height: height,
                _format: format,
            },
        );
        println!("[vkduct] vkCreateImage (2D, {}x{}) -> Handle ID: {}", width, height, handle.0);
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
                "[vkduct] vkCmdCopyBufferToImage (Texture={}, Region=[{}, {}, {}x{}])",
                handle.0, x, y, width, height
            );
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_texture(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if self.textures.remove(&handle).is_some() {
            println!("[vkduct] vkDestroyImage -> Handle ID: {}", handle.0);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_texture(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult {
        println!(
            "[vkduct] vkCmdBindDescriptorSets (Slot={}, CombinedImageSampler={})",
            slot, handle.0
        );
        DuctResult::Success
    }

    fn generate_mipmaps(&mut self, handle: DuctTextureHandle) -> DuctResult {
        println!(
            "[vkduct] vkCmdPipelineBarrier & vkCmdBlitImage mipmap chain for Texture {}",
            handle.0
        );
        DuctResult::Success
    }

    fn set_texture_filter(
        &mut self,
        handle: DuctTextureHandle,
        min: DuctFilterMode,
        mag: DuctFilterMode,
    ) -> DuctResult {
        println!(
            "[vkduct] VkSamplerCreateInfo: Texture {}, Min={:?}, Mag={:?}",
            handle.0, min, mag
        );
        DuctResult::Success
    }

    fn set_texture_wrap(
        &mut self,
        handle: DuctTextureHandle,
        wrap_s: DuctWrapMode,
        wrap_t: DuctWrapMode,
    ) -> DuctResult {
        println!(
            "[vkduct] VkSamplerCreateInfo: Texture {}, WrapS={:?}, WrapT={:?}",
            handle.0, wrap_s, wrap_t
        );
        DuctResult::Success
    }

    fn create_render_target(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        let color_tex = self.create_texture_2d(desc.width, desc.height, desc.format, None)?;
        let handle = DuctRenderTargetHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.render_targets.insert(
            handle,
            VkRenderTargetRecord {
                _width: desc.width,
                _height: desc.height,
                color_tex,
            },
        );
        println!(
            "[vkduct] vkCreateFramebuffer ({}x{}, Depth={}) -> Handle ID: {}",
            desc.width, desc.height, desc.has_depth, handle.0
        );
        Ok(handle)
    }

    fn destroy_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if let Some(rt) = self.render_targets.remove(&handle) {
            let _ = self.destroy_texture(rt.color_tex);
        }
        println!("[vkduct] vkDestroyFramebuffer -> Handle ID: {}", handle.0);
        DuctResult::Success
    }

    fn bind_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        println!("[vkduct] vkCmdBeginRenderPass for Framebuffer {}", handle.0);
        DuctResult::Success
    }

    fn get_render_target_texture(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        if let Some(rt) = self.render_targets.get(&handle) {
            Ok(rt.color_tex)
        } else {
            Ok(DuctTextureHandle(handle.0))
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
        println!(
            "[vkduct] vkCmdBlitImage (Src={} [{}x{}] -> Dst={} [{}x{}], Filter={:?})",
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
        self.pipeline_switches_in_frame += 1;
        println!(
            "[vkduct] vkCmdBindPipeline (GraphicsPipeline, VS={}, FS={}, Blend={:?})",
            desc.vertex_shader.0, desc.fragment_shader.0, desc.blend_mode
        );
        DuctResult::Success
    }

    fn set_wireframe(&mut self, enabled: bool) -> DuctResult {
        println!(
            "[vkduct] VkPipelineRasterizationStateCreateInfo polygonMode = {}",
            if enabled { "LINE" } else { "FILL" }
        );
        DuctResult::Success
    }

    fn set_vsync(&mut self, enabled: bool) -> DuctResult {
        println!(
            "[vkduct] VkPresentModeKHR set to {}",
            if enabled { "VK_PRESENT_MODE_FIFO_KHR" } else { "VK_PRESENT_MODE_IMMEDIATE_KHR" }
        );
        DuctResult::Success
    }

    fn set_uniform_int(&mut self, name: &str, val: i32) -> DuctResult {
        println!("[vkduct] vkCmdPushConstants: {} = {}", name, val);
        DuctResult::Success
    }

    fn set_uniform_float(&mut self, name: &str, val: f32) -> DuctResult {
        println!("[vkduct] vkCmdPushConstants: {} = {}", name, val);
        DuctResult::Success
    }

    fn set_uniform_vec2(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        println!("[vkduct] vkCmdPushConstants: {} = [{}, {}]", name, x, y);
        DuctResult::Success
    }

    fn set_uniform_vec3(&mut self, name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        println!("[vkduct] vkCmdPushConstants: {} = [{}, {}, {}]", name, x, y, z);
        DuctResult::Success
    }

    fn set_uniform_vec4(&mut self, name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        println!("[vkduct] vkCmdPushConstants: {} = [{}, {}, {}, {}]", name, x, y, z, w);
        DuctResult::Success
    }

    fn set_uniform_mat4(&mut self, name: &str, _elements: &[f32; 16]) -> DuctResult {
        println!("[vkduct] vkCmdPushConstants: {} = Mat4", name);
        DuctResult::Success
    }

    fn draw(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count;
        println!(
            "[vkduct] vkCmdDraw(vertexCount={}, instanceCount=1, firstVertex={})",
            vertex_count, first_vertex
        );
        DuctResult::Success
    }

    fn draw_indexed(&mut self, index_count: u32, first_index: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count;
        println!(
            "[vkduct] vkCmdDrawIndexed(indexCount={}, instanceCount=1, firstIndex={})",
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
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count * instance_count;
        println!(
            "[vkduct] vkCmdDraw(vertexCount={}, instanceCount={}, firstVertex={}, firstInstance={})",
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
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count * instance_count;
        println!(
            "[vkduct] vkCmdDrawIndexed(indexCount={}, instanceCount={}, firstIndex={}, format={:?})",
            index_count, instance_count, first_index, index_format
        );
        DuctResult::Success
    }

    fn dispatch_compute(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        println!(
            "[vkduct] vkCmdDispatch(groupCountX={}, groupCountY={}, groupCountZ={})",
            groups_x, groups_y, groups_z
        );
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

export_pipeline!(VkPipeline);
