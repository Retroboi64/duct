//! Zero-dependency Vulkan C-ABI definitions and dynamic function loader for `vkduct`.
//!
//! Loads `vulkan-1.dll` (Windows), `libvulkan.so.1` (Linux), or `libvulkan.1.dylib` /
//! `libMoltenVK.dylib` (macOS) directly via OS primitives without any external crates.

#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::{c_char, c_void, CString};

// ── Basic Vulkan Types & Handles ────────────────────────────────────────────

pub type VkResult = i32;
pub type VkStructureType = i32;
pub type VkFlags = u32;
pub type VkBool32 = u32;
pub type VkDeviceSize = u64;
pub type VkSampleMask = u32;

pub type VkInstance = *mut c_void;
pub type VkPhysicalDevice = *mut c_void;
pub type VkDevice = *mut c_void;
pub type VkQueue = *mut c_void;
pub type VkCommandBuffer = *mut c_void;

pub type VkSurfaceKHR = u64;
pub type VkSwapchainKHR = u64;
pub type VkImage = u64;
pub type VkImageView = u64;
pub type VkShaderModule = u64;
pub type VkBuffer = u64;
pub type VkDeviceMemory = u64;
pub type VkCommandPool = u64;
pub type VkRenderPass = u64;
pub type VkFramebuffer = u64;
pub type VkPipeline = u64;
pub type VkPipelineLayout = u64;
pub type VkSemaphore = u64;
pub type VkFence = u64;

pub const VK_NULL_HANDLE: u64 = 0;
pub const VK_TRUE: VkBool32 = 1;
pub const VK_FALSE: VkBool32 = 0;
pub const VK_WHOLE_SIZE: VkDeviceSize = !0u64;

// ── Result Codes ────────────────────────────────────────────────────────────

pub const VK_SUCCESS: VkResult = 0;
pub const VK_NOT_READY: VkResult = 1;
pub const VK_TIMEOUT: VkResult = 2;
pub const VK_INCOMPLETE: VkResult = 5;
pub const VK_SUBOPTIMAL_KHR: VkResult = 1000001003;
pub const VK_ERROR_OUT_OF_DATE_KHR: VkResult = -1000001004;

// ── API Versions ────────────────────────────────────────────────────────────

#[inline]
pub const fn vk_make_api_version(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
    (variant << 29) | (major << 22) | (minor << 12) | patch
}

pub const VK_API_VERSION_1_0: u32 = vk_make_api_version(0, 1, 0, 0);
pub const VK_API_VERSION_1_2: u32 = vk_make_api_version(0, 1, 2, 0);
pub const VK_API_VERSION_1_3: u32 = vk_make_api_version(0, 1, 3, 0);

// ── Structure Types ─────────────────────────────────────────────────────────

pub const VK_STRUCTURE_TYPE_APPLICATION_INFO: VkStructureType = 0;
pub const VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO: VkStructureType = 1;
pub const VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO: VkStructureType = 2;
pub const VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO: VkStructureType = 3;
pub const VK_STRUCTURE_TYPE_SUBMIT_INFO: VkStructureType = 4;
pub const VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO: VkStructureType = 5;
pub const VK_STRUCTURE_TYPE_FENCE_CREATE_INFO: VkStructureType = 8;
pub const VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO: VkStructureType = 9;
pub const VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO: VkStructureType = 12;
pub const VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO: VkStructureType = 15;
pub const VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO: VkStructureType = 16;
pub const VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO: VkStructureType = 37;
pub const VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO: VkStructureType = 38;
pub const VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO: VkStructureType = 39;
pub const VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO: VkStructureType = 40;
pub const VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO: VkStructureType = 42;
pub const VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO: VkStructureType = 43;
pub const VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR: VkStructureType = 1000001000;
pub const VK_STRUCTURE_TYPE_PRESENT_INFO_KHR: VkStructureType = 1000001001;
pub const VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR: VkStructureType = 1000004000;
pub const VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR: VkStructureType = 1000009000;
pub const VK_STRUCTURE_TYPE_METAL_SURFACE_CREATE_INFO_EXT: VkStructureType = 1000217000;

// ── Enums & Bitflags ────────────────────────────────────────────────────────

pub const VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU: u32 = 1;
pub const VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU: u32 = 2;
pub const VK_PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU: u32 = 3;
pub const VK_PHYSICAL_DEVICE_TYPE_CPU: u32 = 4;

pub const VK_QUEUE_GRAPHICS_BIT: u32 = 0x0000_0001;
pub const VK_QUEUE_COMPUTE_BIT: u32 = 0x0000_0002;
pub const VK_QUEUE_TRANSFER_BIT: u32 = 0x0000_0004;

pub const VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT: u32 = 0x0000_0002;
pub const VK_COMMAND_BUFFER_LEVEL_PRIMARY: u32 = 0;
pub const VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT: u32 = 0x0000_0001;

pub const VK_FENCE_CREATE_SIGNALED_BIT: u32 = 0x0000_0001;

pub const VK_FORMAT_B8G8R8A8_UNORM: u32 = 44;
pub const VK_FORMAT_B8G8R8A8_SRGB: u32 = 50;
pub const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
pub const VK_COLOR_SPACE_SRGB_NONLINEAR_KHR: u32 = 0;

pub const VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT: u32 = 0x0000_0010;
pub const VK_IMAGE_USAGE_TRANSFER_SRC_BIT: u32 = 0x0000_0001;
pub const VK_IMAGE_USAGE_TRANSFER_DST_BIT: u32 = 0x0000_0002;

pub const VK_SHARING_MODE_EXCLUSIVE: u32 = 0;
pub const VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR: u32 = 0x0000_0001;

pub const VK_PRESENT_MODE_IMMEDIATE_KHR: u32 = 0;
pub const VK_PRESENT_MODE_MAILBOX_KHR: u32 = 1;
pub const VK_PRESENT_MODE_FIFO_KHR: u32 = 2;

pub const VK_IMAGE_VIEW_TYPE_2D: u32 = 1;
pub const VK_COMPONENT_SWIZZLE_IDENTITY: u32 = 0;
pub const VK_IMAGE_ASPECT_COLOR_BIT: u32 = 0x0000_0001;

pub const VK_SAMPLE_COUNT_1_BIT: u32 = 0x0000_0001;
pub const VK_ATTACHMENT_LOAD_OP_CLEAR: u32 = 1;
pub const VK_ATTACHMENT_LOAD_OP_DONT_CARE: u32 = 2;
pub const VK_ATTACHMENT_STORE_OP_STORE: u32 = 0;
pub const VK_ATTACHMENT_STORE_OP_DONT_CARE: u32 = 1;

pub const VK_IMAGE_LAYOUT_UNDEFINED: u32 = 0;
pub const VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL: u32 = 2;
pub const VK_IMAGE_LAYOUT_PRESENT_SRC_KHR: u32 = 1000001002;

pub const VK_PIPELINE_BIND_POINT_GRAPHICS: u32 = 0;
pub const VK_PIPELINE_BIND_POINT_COMPUTE: u32 = 1;
pub const VK_SUBPASS_EXTERNAL: u32 = !0u32;
pub const VK_SUBPASS_CONTENTS_INLINE: u32 = 0;

pub const VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT: u32 = 0x0000_0400;
pub const VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT: u32 = 0x0000_0100;

pub const VK_BUFFER_USAGE_TRANSFER_SRC_BIT: u32 = 0x0000_0001;
pub const VK_BUFFER_USAGE_TRANSFER_DST_BIT: u32 = 0x0000_0002;
pub const VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT: u32 = 0x0000_0010;
pub const VK_BUFFER_USAGE_STORAGE_BUFFER_BIT: u32 = 0x0000_0020;
pub const VK_BUFFER_USAGE_INDEX_BUFFER_BIT: u32 = 0x0000_0040;
pub const VK_BUFFER_USAGE_VERTEX_BUFFER_BIT: u32 = 0x0000_0080;

pub const VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT: u32 = 0x0000_0001;
pub const VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT: u32 = 0x0000_0002;
pub const VK_MEMORY_PROPERTY_HOST_COHERENT_BIT: u32 = 0x0000_0004;

pub const VK_INDEX_TYPE_UINT16: u32 = 0;
pub const VK_INDEX_TYPE_UINT32: u32 = 1;

// ── Vulkan Structures ───────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkApplicationInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub pApplicationName: *const c_char,
    pub applicationVersion: u32,
    pub pEngineName: *const c_char,
    pub engineVersion: u32,
    pub apiVersion: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkInstanceCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub pApplicationInfo: *const VkApplicationInfo,
    pub enabledLayerCount: u32,
    pub ppEnabledLayerNames: *const *const c_char,
    pub enabledExtensionCount: u32,
    pub ppEnabledExtensionNames: *const *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkPhysicalDeviceProperties {
    pub apiVersion: u32,
    pub driverVersion: u32,
    pub vendorID: u32,
    pub deviceID: u32,
    pub deviceType: u32,
    pub deviceName: [c_char; 256],
    pub pipelineCacheUUID: [u8; 16],
    pub _limits_and_sparse_padding: [u8; 640],
}

impl Default for VkPhysicalDeviceProperties {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkExtent2D {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkExtent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkOffset2D {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkRect2D {
    pub offset: VkOffset2D,
    pub extent: VkExtent2D,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkViewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub minDepth: f32,
    pub maxDepth: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkQueueFamilyProperties {
    pub queueFlags: VkFlags,
    pub queueCount: u32,
    pub timestampValidBits: u32,
    pub minImageTransferGranularity: VkExtent3D,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkMemoryType {
    pub propertyFlags: VkFlags,
    pub heapIndex: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkMemoryHeap {
    pub size: VkDeviceSize,
    pub flags: VkFlags,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkPhysicalDeviceMemoryProperties {
    pub memoryTypeCount: u32,
    pub memoryTypes: [VkMemoryType; 32],
    pub memoryHeapCount: u32,
    pub memoryHeaps: [VkMemoryHeap; 16],
}

impl Default for VkPhysicalDeviceMemoryProperties {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkDeviceQueueCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub queueFamilyIndex: u32,
    pub queueCount: u32,
    pub pQueuePriorities: *const f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkDeviceCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub queueCreateInfoCount: u32,
    pub pQueueCreateInfos: *const VkDeviceQueueCreateInfo,
    pub enabledLayerCount: u32,
    pub ppEnabledLayerNames: *const *const c_char,
    pub enabledExtensionCount: u32,
    pub ppEnabledExtensionNames: *const *const c_char,
    pub pEnabledFeatures: *const c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkWin32SurfaceCreateInfoKHR {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub hinstance: *mut c_void,
    pub hwnd: *mut c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkXlibSurfaceCreateInfoKHR {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub dpy: *mut c_void,
    pub window: u64,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkMetalSurfaceCreateInfoEXT {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub pLayer: *const c_void,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkSurfaceCapabilitiesKHR {
    pub minImageCount: u32,
    pub maxImageCount: u32,
    pub currentExtent: VkExtent2D,
    pub minImageExtent: VkExtent2D,
    pub maxImageExtent: VkExtent2D,
    pub maxImageArrayLayers: u32,
    pub supportedTransforms: VkFlags,
    pub currentTransform: u32,
    pub supportedCompositeAlpha: VkFlags,
    pub supportedUsageFlags: VkFlags,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkSurfaceFormatKHR {
    pub format: u32,
    pub colorSpace: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkSwapchainCreateInfoKHR {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub surface: VkSurfaceKHR,
    pub minImageCount: u32,
    pub imageFormat: u32,
    pub imageColorSpace: u32,
    pub imageExtent: VkExtent2D,
    pub imageArrayLayers: u32,
    pub imageUsage: VkFlags,
    pub imageSharingMode: u32,
    pub queueFamilyIndexCount: u32,
    pub pQueueFamilyIndices: *const u32,
    pub preTransform: u32,
    pub compositeAlpha: u32,
    pub presentMode: u32,
    pub clipped: VkBool32,
    pub oldSwapchain: VkSwapchainKHR,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkComponentMapping {
    pub r: u32,
    pub g: u32,
    pub b: u32,
    pub a: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkImageSubresourceRange {
    pub aspectMask: VkFlags,
    pub baseMipLevel: u32,
    pub levelCount: u32,
    pub baseArrayLayer: u32,
    pub layerCount: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkImageViewCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub image: VkImage,
    pub viewType: u32,
    pub format: u32,
    pub components: VkComponentMapping,
    pub subresourceRange: VkImageSubresourceRange,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkAttachmentDescription {
    pub flags: VkFlags,
    pub format: u32,
    pub samples: u32,
    pub loadOp: u32,
    pub storeOp: u32,
    pub stencilLoadOp: u32,
    pub stencilStoreOp: u32,
    pub initialLayout: u32,
    pub finalLayout: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkAttachmentReference {
    pub attachment: u32,
    pub layout: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkSubpassDescription {
    pub flags: VkFlags,
    pub pipelineBindPoint: u32,
    pub inputAttachmentCount: u32,
    pub pInputAttachments: *const VkAttachmentReference,
    pub colorAttachmentCount: u32,
    pub pColorAttachments: *const VkAttachmentReference,
    pub pResolveAttachments: *const VkAttachmentReference,
    pub pDepthStencilAttachment: *const VkAttachmentReference,
    pub preserveAttachmentCount: u32,
    pub pPreserveAttachments: *const u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkSubpassDependency {
    pub srcSubpass: u32,
    pub dstSubpass: u32,
    pub srcStageMask: VkFlags,
    pub dstStageMask: VkFlags,
    pub srcAccessMask: VkFlags,
    pub dstAccessMask: VkFlags,
    pub dependencyFlags: VkFlags,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkRenderPassCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub attachmentCount: u32,
    pub pAttachments: *const VkAttachmentDescription,
    pub subpassCount: u32,
    pub pSubpasses: *const VkSubpassDescription,
    pub dependencyCount: u32,
    pub pDependencies: *const VkSubpassDependency,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkFramebufferCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub renderPass: VkRenderPass,
    pub attachmentCount: u32,
    pub pAttachments: *const VkImageView,
    pub width: u32,
    pub height: u32,
    pub layers: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkCommandPoolCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub queueFamilyIndex: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkCommandBufferAllocateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub commandPool: VkCommandPool,
    pub level: u32,
    pub commandBufferCount: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkCommandBufferBeginInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub pInheritanceInfo: *const c_void,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkClearValue {
    pub float32: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkRenderPassBeginInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub renderPass: VkRenderPass,
    pub framebuffer: VkFramebuffer,
    pub renderArea: VkRect2D,
    pub clearValueCount: u32,
    pub pClearValues: *const VkClearValue,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkSubmitInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub waitSemaphoreCount: u32,
    pub pWaitSemaphores: *const VkSemaphore,
    pub pWaitDstStageMask: *const VkFlags,
    pub commandBufferCount: u32,
    pub pCommandBuffers: *const VkCommandBuffer,
    pub signalSemaphoreCount: u32,
    pub pSignalSemaphores: *const VkSemaphore,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkPresentInfoKHR {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub waitSemaphoreCount: u32,
    pub pWaitSemaphores: *const VkSemaphore,
    pub swapchainCount: u32,
    pub pSwapchains: *const VkSwapchainKHR,
    pub pImageIndices: *const u32,
    pub pResults: *mut VkResult,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkFenceCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkSemaphoreCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkBufferCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub size: VkDeviceSize,
    pub usage: VkFlags,
    pub sharingMode: u32,
    pub queueFamilyIndexCount: u32,
    pub pQueueFamilyIndices: *const u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct VkMemoryRequirements {
    pub size: VkDeviceSize,
    pub alignment: VkDeviceSize,
    pub memoryTypeBits: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkMemoryAllocateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub allocationSize: VkDeviceSize,
    pub memoryTypeIndex: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkShaderModuleCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub codeSize: usize,
    pub pCode: *const u32,
}

// ── Dynamic Library & Function Tables ───────────────────────────────────────

pub type PFN_vkGetInstanceProcAddr =
    unsafe extern "system" fn(instance: VkInstance, pName: *const c_char) -> *const c_void;
pub type PFN_vkGetDeviceProcAddr =
    unsafe extern "system" fn(device: VkDevice, pName: *const c_char) -> *const c_void;

#[derive(Copy, Clone)]
pub struct VkInstanceFns {
    pub DestroyInstance: unsafe extern "system" fn(instance: VkInstance, pAllocator: *const c_void),
    pub EnumeratePhysicalDevices: unsafe extern "system" fn(
        instance: VkInstance,
        pPhysicalDeviceCount: *mut u32,
        pPhysicalDevices: *mut VkPhysicalDevice,
    ) -> VkResult,
    pub GetPhysicalDeviceProperties: unsafe extern "system" fn(
        physicalDevice: VkPhysicalDevice,
        pProperties: *mut VkPhysicalDeviceProperties,
    ),
    pub GetPhysicalDeviceQueueFamilyProperties: unsafe extern "system" fn(
        physicalDevice: VkPhysicalDevice,
        pQueueFamilyPropertyCount: *mut u32,
        pQueueFamilyProperties: *mut VkQueueFamilyProperties,
    ),
    pub GetPhysicalDeviceMemoryProperties: unsafe extern "system" fn(
        physicalDevice: VkPhysicalDevice,
        pMemoryProperties: *mut VkPhysicalDeviceMemoryProperties,
    ),
    pub CreateDevice: unsafe extern "system" fn(
        physicalDevice: VkPhysicalDevice,
        pCreateInfo: *const VkDeviceCreateInfo,
        pAllocator: *const c_void,
        pDevice: *mut VkDevice,
    ) -> VkResult,
    pub GetDeviceProcAddr: PFN_vkGetDeviceProcAddr,

    // Surface KHR functions (Optional if surface extension is active)
    pub DestroySurfaceKHR: Option<
        unsafe extern "system" fn(instance: VkInstance, surface: VkSurfaceKHR, pAllocator: *const c_void),
    >,
    pub GetPhysicalDeviceSurfaceSupportKHR: Option<
        unsafe extern "system" fn(
            physicalDevice: VkPhysicalDevice,
            queueFamilyIndex: u32,
            surface: VkSurfaceKHR,
            pSupported: *mut VkBool32,
        ) -> VkResult,
    >,
    pub GetPhysicalDeviceSurfaceCapabilitiesKHR: Option<
        unsafe extern "system" fn(
            physicalDevice: VkPhysicalDevice,
            surface: VkSurfaceKHR,
            pSurfaceCapabilities: *mut VkSurfaceCapabilitiesKHR,
        ) -> VkResult,
    >,
    pub GetPhysicalDeviceSurfaceFormatsKHR: Option<
        unsafe extern "system" fn(
            physicalDevice: VkPhysicalDevice,
            surface: VkSurfaceKHR,
            pSurfaceFormatCount: *mut u32,
            pSurfaceFormats: *mut VkSurfaceFormatKHR,
        ) -> VkResult,
    >,
    pub CreateWin32SurfaceKHR: Option<
        unsafe extern "system" fn(
            instance: VkInstance,
            pCreateInfo: *const VkWin32SurfaceCreateInfoKHR,
            pAllocator: *const c_void,
            pSurface: *mut VkSurfaceKHR,
        ) -> VkResult,
    >,
    pub CreateXlibSurfaceKHR: Option<
        unsafe extern "system" fn(
            instance: VkInstance,
            pCreateInfo: *const VkXlibSurfaceCreateInfoKHR,
            pAllocator: *const c_void,
            pSurface: *mut VkSurfaceKHR,
        ) -> VkResult,
    >,
}

#[derive(Copy, Clone)]
pub struct VkDeviceFns {
    pub DestroyDevice: unsafe extern "system" fn(device: VkDevice, pAllocator: *const c_void),
    pub DeviceWaitIdle: unsafe extern "system" fn(device: VkDevice) -> VkResult,
    pub GetDeviceQueue: unsafe extern "system" fn(
        device: VkDevice,
        queueFamilyIndex: u32,
        queueIndex: u32,
        pQueue: *mut VkQueue,
    ),
    pub QueueSubmit: unsafe extern "system" fn(
        queue: VkQueue,
        submitCount: u32,
        pSubmits: *const VkSubmitInfo,
        fence: VkFence,
    ) -> VkResult,
    pub QueueWaitIdle: unsafe extern "system" fn(queue: VkQueue) -> VkResult,

    // Command Pools & Buffers
    pub CreateCommandPool: unsafe extern "system" fn(
        device: VkDevice,
        pCreateInfo: *const VkCommandPoolCreateInfo,
        pAllocator: *const c_void,
        pCommandPool: *mut VkCommandPool,
    ) -> VkResult,
    pub DestroyCommandPool: unsafe extern "system" fn(
        device: VkDevice,
        commandPool: VkCommandPool,
        pAllocator: *const c_void,
    ),
    pub AllocateCommandBuffers: unsafe extern "system" fn(
        device: VkDevice,
        pAllocateInfo: *const VkCommandBufferAllocateInfo,
        pCommandBuffers: *mut VkCommandBuffer,
    ) -> VkResult,
    pub FreeCommandBuffers: unsafe extern "system" fn(
        device: VkDevice,
        commandPool: VkCommandPool,
        commandBufferCount: u32,
        pCommandBuffers: *const VkCommandBuffer,
    ),
    pub ResetCommandBuffer: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        flags: VkFlags,
    ) -> VkResult,
    pub BeginCommandBuffer: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        pBeginInfo: *const VkCommandBufferBeginInfo,
    ) -> VkResult,
    pub EndCommandBuffer: unsafe extern "system" fn(commandBuffer: VkCommandBuffer) -> VkResult,

    // RenderPass & Framebuffer
    pub CreateRenderPass: unsafe extern "system" fn(
        device: VkDevice,
        pCreateInfo: *const VkRenderPassCreateInfo,
        pAllocator: *const c_void,
        pRenderPass: *mut VkRenderPass,
    ) -> VkResult,
    pub DestroyRenderPass: unsafe extern "system" fn(
        device: VkDevice,
        renderPass: VkRenderPass,
        pAllocator: *const c_void,
    ),
    pub CreateFramebuffer: unsafe extern "system" fn(
        device: VkDevice,
        pCreateInfo: *const VkFramebufferCreateInfo,
        pAllocator: *const c_void,
        pFramebuffer: *mut VkFramebuffer,
    ) -> VkResult,
    pub DestroyFramebuffer: unsafe extern "system" fn(
        device: VkDevice,
        framebuffer: VkFramebuffer,
        pAllocator: *const c_void,
    ),
    pub CreateImageView: unsafe extern "system" fn(
        device: VkDevice,
        pCreateInfo: *const VkImageViewCreateInfo,
        pAllocator: *const c_void,
        pView: *mut VkImageView,
    ) -> VkResult,
    pub DestroyImageView: unsafe extern "system" fn(
        device: VkDevice,
        imageView: VkImageView,
        pAllocator: *const c_void,
    ),

    // Command Recording
    pub CmdBeginRenderPass: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        pRenderPassBegin: *const VkRenderPassBeginInfo,
        contents: u32,
    ),
    pub CmdEndRenderPass: unsafe extern "system" fn(commandBuffer: VkCommandBuffer),
    pub CmdSetViewport: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        firstViewport: u32,
        viewportCount: u32,
        pViewports: *const VkViewport,
    ),
    pub CmdSetScissor: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        firstScissor: u32,
        scissorCount: u32,
        pScissors: *const VkRect2D,
    ),
    pub CmdBindVertexBuffers: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        firstBinding: u32,
        bindingCount: u32,
        pBuffers: *const VkBuffer,
        pOffsets: *const VkDeviceSize,
    ),
    pub CmdBindIndexBuffer: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        buffer: VkBuffer,
        offset: VkDeviceSize,
        indexType: u32,
    ),
    pub CmdDraw: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        vertexCount: u32,
        instanceCount: u32,
        firstVertex: u32,
        firstInstance: u32,
    ),
    pub CmdDrawIndexed: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        indexCount: u32,
        instanceCount: u32,
        firstIndex: u32,
        vertexOffset: i32,
        firstInstance: u32,
    ),
    pub CmdDispatch: unsafe extern "system" fn(
        commandBuffer: VkCommandBuffer,
        groupCountX: u32,
        groupCountY: u32,
        groupCountZ: u32,
    ),

    // Buffers, Memory & Shaders
    pub CreateBuffer: unsafe extern "system" fn(
        device: VkDevice,
        pCreateInfo: *const VkBufferCreateInfo,
        pAllocator: *const c_void,
        pBuffer: *mut VkBuffer,
    ) -> VkResult,
    pub DestroyBuffer: unsafe extern "system" fn(
        device: VkDevice,
        buffer: VkBuffer,
        pAllocator: *const c_void,
    ),
    pub GetBufferMemoryRequirements: unsafe extern "system" fn(
        device: VkDevice,
        buffer: VkBuffer,
        pMemoryRequirements: *mut VkMemoryRequirements,
    ),
    pub AllocateMemory: unsafe extern "system" fn(
        device: VkDevice,
        pAllocateInfo: *const VkMemoryAllocateInfo,
        pAllocator: *const c_void,
        pMemory: *mut VkDeviceMemory,
    ) -> VkResult,
    pub FreeMemory: unsafe extern "system" fn(
        device: VkDevice,
        memory: VkDeviceMemory,
        pAllocator: *const c_void,
    ),
    pub BindBufferMemory: unsafe extern "system" fn(
        device: VkDevice,
        buffer: VkBuffer,
        memory: VkDeviceMemory,
        memoryOffset: VkDeviceSize,
    ) -> VkResult,
    pub MapMemory: unsafe extern "system" fn(
        device: VkDevice,
        memory: VkDeviceMemory,
        offset: VkDeviceSize,
        size: VkDeviceSize,
        flags: VkFlags,
        ppData: *mut *mut c_void,
    ) -> VkResult,
    pub UnmapMemory: unsafe extern "system" fn(device: VkDevice, memory: VkDeviceMemory),
    pub CreateShaderModule: unsafe extern "system" fn(
        device: VkDevice,
        pCreateInfo: *const VkShaderModuleCreateInfo,
        pAllocator: *const c_void,
        pShaderModule: *mut VkShaderModule,
    ) -> VkResult,
    pub DestroyShaderModule: unsafe extern "system" fn(
        device: VkDevice,
        shaderModule: VkShaderModule,
        pAllocator: *const c_void,
    ),

    // Sync primitives
    pub CreateFence: unsafe extern "system" fn(
        device: VkDevice,
        pCreateInfo: *const VkFenceCreateInfo,
        pAllocator: *const c_void,
        pFence: *mut VkFence,
    ) -> VkResult,
    pub DestroyFence: unsafe extern "system" fn(
        device: VkDevice,
        fence: VkFence,
        pAllocator: *const c_void,
    ),
    pub WaitForFences: unsafe extern "system" fn(
        device: VkDevice,
        fenceCount: u32,
        pFences: *const VkFence,
        waitAll: VkBool32,
        timeout: u64,
    ) -> VkResult,
    pub ResetFences: unsafe extern "system" fn(
        device: VkDevice,
        fenceCount: u32,
        pFences: *const VkFence,
    ) -> VkResult,
    pub CreateSemaphore: unsafe extern "system" fn(
        device: VkDevice,
        pCreateInfo: *const VkSemaphoreCreateInfo,
        pAllocator: *const c_void,
        pSemaphore: *mut VkSemaphore,
    ) -> VkResult,
    pub DestroySemaphore: unsafe extern "system" fn(
        device: VkDevice,
        semaphore: VkSemaphore,
        pAllocator: *const c_void,
    ),

    // Swapchain KHR (Optional if swapchain extension is enabled)
    pub CreateSwapchainKHR: Option<
        unsafe extern "system" fn(
            device: VkDevice,
            pCreateInfo: *const VkSwapchainCreateInfoKHR,
            pAllocator: *const c_void,
            pSwapchain: *mut VkSwapchainKHR,
        ) -> VkResult,
    >,
    pub DestroySwapchainKHR: Option<
        unsafe extern "system" fn(
            device: VkDevice,
            swapchain: VkSwapchainKHR,
            pAllocator: *const c_void,
        ),
    >,
    pub GetSwapchainImagesKHR: Option<
        unsafe extern "system" fn(
            device: VkDevice,
            swapchain: VkSwapchainKHR,
            pSwapchainImageCount: *mut u32,
            pSwapchainImages: *mut VkImage,
        ) -> VkResult,
    >,
    pub AcquireNextImageKHR: Option<
        unsafe extern "system" fn(
            device: VkDevice,
            swapchain: VkSwapchainKHR,
            timeout: u64,
            semaphore: VkSemaphore,
            fence: VkFence,
            pImageIndex: *mut u32,
        ) -> VkResult,
    >,
    pub QueuePresentKHR: Option<
        unsafe extern "system" fn(queue: VkQueue, pPresentInfo: *const VkPresentInfoKHR) -> VkResult,
    >,
}

// ── Zero-Dependency OS Vulkan Loader ────────────────────────────────────────

pub struct VulkanLoader {
    lib_handle: *mut c_void,
    pub get_instance_proc_addr: PFN_vkGetInstanceProcAddr,
    pub create_instance: unsafe extern "system" fn(
        pCreateInfo: *const VkInstanceCreateInfo,
        pAllocator: *const c_void,
        pInstance: *mut VkInstance,
    ) -> VkResult,
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryA(lpLibFileName: *const c_char) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *const c_void;
    fn FreeLibrary(hLibModule: *mut c_void) -> i32;
}

#[cfg(not(target_os = "windows"))]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
}

macro_rules! load_vk_req {
    ($loader:expr, $ctx:expr, $name:literal, $t:ty) => {{
        let c_str = CString::new($name).unwrap();
        let ptr = ($loader)($ctx, c_str.as_ptr());
        if ptr.is_null() {
            return None;
        }
        std::mem::transmute::<*const c_void, $t>(ptr)
    }};
}

macro_rules! load_vk_opt {
    ($loader:expr, $ctx:expr, $name:literal, $t:ty) => {{
        let c_str = CString::new($name).unwrap();
        let ptr = ($loader)($ctx, c_str.as_ptr());
        if ptr.is_null() {
            None
        } else {
            Some(std::mem::transmute::<*const c_void, $t>(ptr))
        }
    }};
}

impl VulkanLoader {
    pub unsafe fn new() -> Option<Self> {
        #[cfg(target_os = "windows")]
        let lib_handle = unsafe { LoadLibraryA(b"vulkan-1.dll\0".as_ptr() as *const c_char) };

        #[cfg(target_os = "linux")]
        let lib_handle = unsafe {
            let h = dlopen(b"libvulkan.so.1\0".as_ptr() as *const c_char, 2);
            if !h.is_null() {
                h
            } else {
                dlopen(b"libvulkan.so\0".as_ptr() as *const c_char, 2)
            }
        };

        #[cfg(target_os = "macos")]
        let lib_handle = unsafe {
            let h = dlopen(b"libvulkan.1.dylib\0".as_ptr() as *const c_char, 2);
            if !h.is_null() {
                h
            } else {
                dlopen(b"libMoltenVK.dylib\0".as_ptr() as *const c_char, 2)
            }
        };

        if lib_handle.is_null() {
            return None;
        }

        #[cfg(target_os = "windows")]
        let gipa_ptr = unsafe {
            GetProcAddress(lib_handle, b"vkGetInstanceProcAddr\0".as_ptr() as *const c_char)
        };

        #[cfg(not(target_os = "windows"))]
        let gipa_ptr = unsafe {
            dlsym(lib_handle, b"vkGetInstanceProcAddr\0".as_ptr() as *const c_char)
        };

        if gipa_ptr.is_null() {
            return None;
        }

        let get_instance_proc_addr: PFN_vkGetInstanceProcAddr =
            unsafe { std::mem::transmute(gipa_ptr) };

        let create_instance = unsafe {
            load_vk_req!(
                get_instance_proc_addr,
                std::ptr::null_mut(),
                "vkCreateInstance",
                unsafe extern "system" fn(
                    *const VkInstanceCreateInfo,
                    *const c_void,
                    *mut VkInstance,
                ) -> VkResult
            )
        };

        Some(Self {
            lib_handle,
            get_instance_proc_addr,
            create_instance,
        })
    }

    pub unsafe fn load_instance_fns(&self, instance: VkInstance) -> Option<VkInstanceFns> {
        let gipa = self.get_instance_proc_addr;
        unsafe {
            Some(VkInstanceFns {
                DestroyInstance: load_vk_req!(gipa, instance, "vkDestroyInstance", _),
                EnumeratePhysicalDevices: load_vk_req!(gipa, instance, "vkEnumeratePhysicalDevices", _),
                GetPhysicalDeviceProperties: load_vk_req!(gipa, instance, "vkGetPhysicalDeviceProperties", _),
                GetPhysicalDeviceQueueFamilyProperties: load_vk_req!(gipa, instance, "vkGetPhysicalDeviceQueueFamilyProperties", _),
                GetPhysicalDeviceMemoryProperties: load_vk_req!(gipa, instance, "vkGetPhysicalDeviceMemoryProperties", _),
                CreateDevice: load_vk_req!(gipa, instance, "vkCreateDevice", _),
                GetDeviceProcAddr: load_vk_req!(gipa, instance, "vkGetDeviceProcAddr", _),
                DestroySurfaceKHR: load_vk_opt!(gipa, instance, "vkDestroySurfaceKHR", _),
                GetPhysicalDeviceSurfaceSupportKHR: load_vk_opt!(gipa, instance, "vkGetPhysicalDeviceSurfaceSupportKHR", _),
                GetPhysicalDeviceSurfaceCapabilitiesKHR: load_vk_opt!(gipa, instance, "vkGetPhysicalDeviceSurfaceCapabilitiesKHR", _),
                GetPhysicalDeviceSurfaceFormatsKHR: load_vk_opt!(gipa, instance, "vkGetPhysicalDeviceSurfaceFormatsKHR", _),
                CreateWin32SurfaceKHR: load_vk_opt!(gipa, instance, "vkCreateWin32SurfaceKHR", _),
                CreateXlibSurfaceKHR: load_vk_opt!(gipa, instance, "vkCreateXlibSurfaceKHR", _),
            })
        }
    }

    pub unsafe fn load_device_fns(
        &self,
        instance_fns: &VkInstanceFns,
        device: VkDevice,
    ) -> Option<VkDeviceFns> {
        let gdpa = instance_fns.GetDeviceProcAddr;
        unsafe {
            Some(VkDeviceFns {
                DestroyDevice: load_vk_req!(gdpa, device, "vkDestroyDevice", _),
                DeviceWaitIdle: load_vk_req!(gdpa, device, "vkDeviceWaitIdle", _),
                GetDeviceQueue: load_vk_req!(gdpa, device, "vkGetDeviceQueue", _),
                QueueSubmit: load_vk_req!(gdpa, device, "vkQueueSubmit", _),
                QueueWaitIdle: load_vk_req!(gdpa, device, "vkQueueWaitIdle", _),

                CreateCommandPool: load_vk_req!(gdpa, device, "vkCreateCommandPool", _),
                DestroyCommandPool: load_vk_req!(gdpa, device, "vkDestroyCommandPool", _),
                AllocateCommandBuffers: load_vk_req!(gdpa, device, "vkAllocateCommandBuffers", _),
                FreeCommandBuffers: load_vk_req!(gdpa, device, "vkFreeCommandBuffers", _),
                ResetCommandBuffer: load_vk_req!(gdpa, device, "vkResetCommandBuffer", _),
                BeginCommandBuffer: load_vk_req!(gdpa, device, "vkBeginCommandBuffer", _),
                EndCommandBuffer: load_vk_req!(gdpa, device, "vkEndCommandBuffer", _),

                CreateRenderPass: load_vk_req!(gdpa, device, "vkCreateRenderPass", _),
                DestroyRenderPass: load_vk_req!(gdpa, device, "vkDestroyRenderPass", _),
                CreateFramebuffer: load_vk_req!(gdpa, device, "vkCreateFramebuffer", _),
                DestroyFramebuffer: load_vk_req!(gdpa, device, "vkDestroyFramebuffer", _),
                CreateImageView: load_vk_req!(gdpa, device, "vkCreateImageView", _),
                DestroyImageView: load_vk_req!(gdpa, device, "vkDestroyImageView", _),

                CmdBeginRenderPass: load_vk_req!(gdpa, device, "vkCmdBeginRenderPass", _),
                CmdEndRenderPass: load_vk_req!(gdpa, device, "vkCmdEndRenderPass", _),
                CmdSetViewport: load_vk_req!(gdpa, device, "vkCmdSetViewport", _),
                CmdSetScissor: load_vk_req!(gdpa, device, "vkCmdSetScissor", _),
                CmdBindVertexBuffers: load_vk_req!(gdpa, device, "vkCmdBindVertexBuffers", _),
                CmdBindIndexBuffer: load_vk_req!(gdpa, device, "vkCmdBindIndexBuffer", _),
                CmdDraw: load_vk_req!(gdpa, device, "vkCmdDraw", _),
                CmdDrawIndexed: load_vk_req!(gdpa, device, "vkCmdDrawIndexed", _),
                CmdDispatch: load_vk_req!(gdpa, device, "vkCmdDispatch", _),

                CreateBuffer: load_vk_req!(gdpa, device, "vkCreateBuffer", _),
                DestroyBuffer: load_vk_req!(gdpa, device, "vkDestroyBuffer", _),
                GetBufferMemoryRequirements: load_vk_req!(gdpa, device, "vkGetBufferMemoryRequirements", _),
                AllocateMemory: load_vk_req!(gdpa, device, "vkAllocateMemory", _),
                FreeMemory: load_vk_req!(gdpa, device, "vkFreeMemory", _),
                BindBufferMemory: load_vk_req!(gdpa, device, "vkBindBufferMemory", _),
                MapMemory: load_vk_req!(gdpa, device, "vkMapMemory", _),
                UnmapMemory: load_vk_req!(gdpa, device, "vkUnmapMemory", _),
                CreateShaderModule: load_vk_req!(gdpa, device, "vkCreateShaderModule", _),
                DestroyShaderModule: load_vk_req!(gdpa, device, "vkDestroyShaderModule", _),

                CreateFence: load_vk_req!(gdpa, device, "vkCreateFence", _),
                DestroyFence: load_vk_req!(gdpa, device, "vkDestroyFence", _),
                WaitForFences: load_vk_req!(gdpa, device, "vkWaitForFences", _),
                ResetFences: load_vk_req!(gdpa, device, "vkResetFences", _),
                CreateSemaphore: load_vk_req!(gdpa, device, "vkCreateSemaphore", _),
                DestroySemaphore: load_vk_req!(gdpa, device, "vkDestroySemaphore", _),

                CreateSwapchainKHR: load_vk_opt!(gdpa, device, "vkCreateSwapchainKHR", _),
                DestroySwapchainKHR: load_vk_opt!(gdpa, device, "vkDestroySwapchainKHR", _),
                GetSwapchainImagesKHR: load_vk_opt!(gdpa, device, "vkGetSwapchainImagesKHR", _),
                AcquireNextImageKHR: load_vk_opt!(gdpa, device, "vkAcquireNextImageKHR", _),
                QueuePresentKHR: load_vk_opt!(gdpa, device, "vkQueuePresentKHR", _),
            })
        }
    }
}

impl Drop for VulkanLoader {
    fn drop(&mut self) {
        if !self.lib_handle.is_null() {
            #[cfg(target_os = "windows")]
            unsafe {
                FreeLibrary(self.lib_handle);
            }
            #[cfg(not(target_os = "windows"))]
            unsafe {
                dlclose(self.lib_handle);
            }
            self.lib_handle = std::ptr::null_mut();
        }
    }
}
