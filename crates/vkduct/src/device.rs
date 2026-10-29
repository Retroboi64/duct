use crate::spv;
use crate::types::*;
use crate::types::VkPipeline;
use crate::vk::*;
use duct_core::*;
use std::ffi::{c_char, CStr, CString};

impl VkPipeline {
    pub(crate) fn find_memory_type(&self, type_filter: u32, properties: VkFlags) -> Option<u32> {
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

    pub(crate) unsafe fn create_spv_module(&self, words: &[u32]) -> VkShaderModule {
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

    pub(crate) unsafe fn create_single_render_pass(
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

    pub(crate) unsafe fn cleanup_swapchain(&mut self) {
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

    pub(crate) unsafe fn init_swapchain(&mut self, width: u32, height: u32, vsync: bool) -> bool {
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

    pub(crate) fn init_impl(&mut self, config: &DuctInitConfig) -> DuctResult {
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

                                // Create Descriptor Set Layout for Textures & Samplers (up to 8 slots)
                                let mut dsl_bindings = Vec::new();
                                for i in 0..8u32 {
                                    // Separated Sampled Image (binding = 2*i)
                                    dsl_bindings.push(VkDescriptorSetLayoutBinding {
                                        binding: 2 * i,
                                        descriptorType: VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                                        descriptorCount: 1,
                                        stageFlags: VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
                                        pImmutableSamplers: std::ptr::null(),
                                    });
                                    // Separated Sampler (binding = 2*i + 1)
                                    dsl_bindings.push(VkDescriptorSetLayoutBinding {
                                        binding: 2 * i + 1,
                                        descriptorType: VK_DESCRIPTOR_TYPE_SAMPLER,
                                        descriptorCount: 1,
                                        stageFlags: VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
                                        pImmutableSamplers: std::ptr::null(),
                                    });
                                    // Combined Image Sampler (binding = 16 + i)
                                    dsl_bindings.push(VkDescriptorSetLayoutBinding {
                                        binding: 16 + i,
                                        descriptorType: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                                        descriptorCount: 1,
                                        stageFlags: VK_SHADER_STAGE_VERTEX_BIT | VK_SHADER_STAGE_FRAGMENT_BIT,
                                        pImmutableSamplers: std::ptr::null(),
                                    });
                                }
                                let dsl_info = VkDescriptorSetLayoutCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                    bindingCount: dsl_bindings.len() as u32,
                                    pBindings: dsl_bindings.as_ptr(),
                                };
                                unsafe {
                                    let _ = (dfns.CreateDescriptorSetLayout)(
                                        device,
                                        &dsl_info,
                                        std::ptr::null(),
                                        &mut self.descriptor_set_layout,
                                    );
                                }

                                // Create Descriptor Pool
                                let pool_sizes = [
                                    VkDescriptorPoolSize {
                                        r#type: VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                                        descriptorCount: 64,
                                    },
                                    VkDescriptorPoolSize {
                                        r#type: VK_DESCRIPTOR_TYPE_SAMPLER,
                                        descriptorCount: 64,
                                    },
                                    VkDescriptorPoolSize {
                                        r#type: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                                        descriptorCount: 64,
                                    },
                                ];
                                let pool_info = VkDescriptorPoolCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                    maxSets: 16,
                                    poolSizeCount: pool_sizes.len() as u32,
                                    pPoolSizes: pool_sizes.as_ptr(),
                                };
                                unsafe {
                                    let _ = (dfns.CreateDescriptorPool)(
                                        device,
                                        &pool_info,
                                        std::ptr::null(),
                                        &mut self.descriptor_pool,
                                    );
                                }

                                // Allocate initial Descriptor Set
                                if self.descriptor_pool != VK_NULL_HANDLE
                                    && self.descriptor_set_layout != VK_NULL_HANDLE
                                {
                                    let alloc_info = VkDescriptorSetAllocateInfo {
                                        sType: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
                                        pNext: std::ptr::null(),
                                        descriptorPool: self.descriptor_pool,
                                        descriptorSetCount: 1,
                                        pSetLayouts: &self.descriptor_set_layout,
                                    };
                                    unsafe {
                                        let _ = (dfns.AllocateDescriptorSets)(
                                            device,
                                            &alloc_info,
                                            &mut self.descriptor_set,
                                        );
                                    }
                                }

                                // Create Pipeline Layouts (Graphics with Push Constants and Descriptor Sets, and Compute)
                                let pc_range = VkPushConstantRange {
                                    stageFlags: VK_SHADER_STAGE_VERTEX_BIT
                                        | VK_SHADER_STAGE_FRAGMENT_BIT,
                                    offset: 0,
                                    size: std::mem::size_of::<VkPushConstantBlock>() as u32,
                                };
                                let set_layouts = [self.descriptor_set_layout];
                                let layout_info = VkPipelineLayoutCreateInfo {
                                    sType: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
                                    pNext: std::ptr::null(),
                                    flags: 0,
                                    setLayoutCount: if self.descriptor_set_layout != VK_NULL_HANDLE { 1 } else { 0 },
                                    pSetLayouts: set_layouts.as_ptr(),
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

                                    let white_bytes: [u8; 4] = [255, 255, 255, 255];
                                    if let Ok(tex) = self.create_texture_2d(1, 1, DuctTextureFormat::Rgba8, Some(&white_bytes)) {
                                        self.default_white_tex = tex;
                                        self.descriptor_set_dirty = true;
                                        self.update_descriptor_set();
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

    pub(crate) fn shutdown_impl(&mut self) -> DuctResult {
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

                    if self.descriptor_pool != VK_NULL_HANDLE {
                        (dfns.DestroyDescriptorPool)(self.device, self.descriptor_pool, std::ptr::null());
                        self.descriptor_pool = VK_NULL_HANDLE;
                        self.descriptor_set = VK_NULL_HANDLE;
                    }
                    if self.descriptor_set_layout != VK_NULL_HANDLE {
                        (dfns.DestroyDescriptorSetLayout)(self.device, self.descriptor_set_layout, std::ptr::null());
                        self.descriptor_set_layout = VK_NULL_HANDLE;
                    }
                    if self.pipeline_layout != VK_NULL_HANDLE {
                        (dfns.DestroyPipelineLayout)(self.device, self.pipeline_layout, std::ptr::null());
                        self.pipeline_layout = VK_NULL_HANDLE;
                    }
                    if self.compute_pipeline_layout != VK_NULL_HANDLE {
                        (dfns.DestroyPipelineLayout)(self.device, self.compute_pipeline_layout, std::ptr::null());
                        self.compute_pipeline_layout = VK_NULL_HANDLE;
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

    pub(crate) fn resize_viewport_impl(&mut self, width: u32, height: u32) -> DuctResult {
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

    pub(crate) fn set_vsync_impl(&mut self, enabled: bool) -> DuctResult {
        self.vsync_enabled = enabled;
        DuctResult::Success
    }

}
