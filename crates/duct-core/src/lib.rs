//! Duct Core: C-ABI plugin interface and shared types for Project32 duct graphics subsystem.
//!
//! Because Rust's dynamic trait objects (`dyn Trait`) have no stable ABI across
//! dynamic library boundaries, this crate defines standard `#[repr(C)]` data structures
//! and function pointer tables (vtables) to guarantee binary compatibility across
//! compilers, profiles, and runtime plugin loads.

use std::ffi::{c_char, c_void};
use std::fmt;

pub mod math;
pub use math::*;

/// Result status codes returned across the C-ABI boundary.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctResult {
    Success = 0,
    ErrorGeneric = -1,
    ErrorInvalidParameter = -2,
    ErrorUnsupported = -3,
    ErrorInitializationFailed = -4,
    ErrorDeviceLost = -5,
    ErrorResourceNotFound = -6,
    ErrorOutOfMemory = -7,
    ErrorInvalidState = -8,
    ErrorWindowRequired = -9,
}

impl DuctResult {
    #[inline]
    pub fn is_ok(self) -> bool {
        self == DuctResult::Success
    }

    #[inline]
    pub fn is_err(self) -> bool {
        self != DuctResult::Success
    }
}

impl fmt::Display for DuctResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DuctResult::Success => write!(f, "Success"),
            DuctResult::ErrorGeneric => write!(f, "Generic Error"),
            DuctResult::ErrorInvalidParameter => write!(f, "Invalid Parameter"),
            DuctResult::ErrorUnsupported => write!(f, "Operation Unsupported"),
            DuctResult::ErrorInitializationFailed => write!(f, "Initialization Failed"),
            DuctResult::ErrorDeviceLost => write!(f, "Device Lost"),
            DuctResult::ErrorResourceNotFound => write!(f, "Resource Not Found"),
            DuctResult::ErrorOutOfMemory => write!(f, "Out of Memory"),
            DuctResult::ErrorInvalidState => write!(f, "Invalid State"),
            DuctResult::ErrorWindowRequired => write!(f, "Attached Native Window Required"),
        }
    }
}

impl std::error::Error for DuctResult {}

/// Graphics backend identifier.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DuctBackendType {
    Unknown = 0,
    OpenGL = 1,
    DirectX12 = 2,
    Vulkan = 3,
    Metal = 4,
    DirectX11 = 5,
    Custom = 99,
}

impl fmt::Display for DuctBackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DuctBackendType::Unknown => write!(f, "Unknown"),
            DuctBackendType::OpenGL => write!(f, "OpenGL"),
            DuctBackendType::DirectX12 => write!(f, "DirectX 12"),
            DuctBackendType::Vulkan => write!(f, "Vulkan"),
            DuctBackendType::Metal => write!(f, "Metal"),
            DuctBackendType::DirectX11 => write!(f, "DirectX 11"),
            DuctBackendType::Custom => write!(f, "Custom"),
        }
    }
}

/// Normalized Device Coordinate (NDC) clip space convention used by a graphics backend.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DuctClipSpace {
    /// OpenGL convention: Depth is mapped to [-1, 1], Y points up.
    OpenGL = 1,
    /// Direct3D convention: Depth is mapped to [0, 1], Y points up.
    Direct3D = 2,
    /// Vulkan convention: Depth is mapped to [0, 1], Y points down in framebuffer space (or compensated via negative viewport).
    Vulkan = 3,
}

impl DuctClipSpace {
    /// Returns true if the depth range is [0, 1] (Direct3D and Vulkan).
    #[inline]
    pub fn is_zero_to_one(self) -> bool {
        matches!(self, DuctClipSpace::Direct3D | DuctClipSpace::Vulkan)
    }
}

impl DuctBackendType {
    /// Returns the NDC clip space convention for this backend type.
    #[inline]
    pub fn clip_space(self) -> DuctClipSpace {
        match self {
            DuctBackendType::OpenGL => DuctClipSpace::OpenGL,
            DuctBackendType::DirectX11 | DuctBackendType::DirectX12 => DuctClipSpace::Direct3D,
            DuctBackendType::Vulkan => DuctClipSpace::Vulkan,
            _ => DuctClipSpace::OpenGL,
        }
    }
}

/// RGBA floating-point color.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DuctColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl DuctColor {
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Self = Self { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const YELLOW: Self = Self { r: 1.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const CYAN: Self = Self { r: 0.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const MAGENTA: Self = Self { r: 1.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const ORANGE: Self = Self { r: 1.0, g: 0.55, b: 0.0, a: 1.0 };
    pub const GRAY: Self = Self { r: 0.5, g: 0.5, b: 0.5, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    pub const CORNFLOWER_BLUE: Self = Self { r: 0.392, g: 0.584, b: 0.929, a: 1.0 };

    #[inline]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    #[inline]
    pub const fn from_rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: (r as f32) / 255.0,
            g: (g as f32) / 255.0,
            b: (b as f32) / 255.0,
            a: (a as f32) / 255.0,
        }
    }

    #[inline]
    pub fn to_rgba_u8(self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    #[inline]
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// Opaque window handle across platforms (compatible with raw window handles or custom managers like windowed).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DuctWindowHandle {
    /// E.g. HWND on Windows, Window XID on X11, NSView on macOS.
    pub native_window: *mut c_void,
    /// E.g. HINSTANCE on Windows, Display on X11.
    pub native_display: *mut c_void,
}

impl DuctWindowHandle {
    #[inline]
    pub const fn null() -> Self {
        Self {
            native_window: std::ptr::null_mut(),
            native_display: std::ptr::null_mut(),
        }
    }

    #[inline]
    pub fn from_raw(native_window: *mut c_void, native_display: *mut c_void) -> Self {
        Self {
            native_window,
            native_display,
        }
    }

    #[inline]
    pub fn from_win32(hwnd: *mut c_void, hinstance: *mut c_void) -> Self {
        Self {
            native_window: hwnd,
            native_display: hinstance,
        }
    }

    #[inline]
    pub fn from_xlib(window: u64, display: *mut c_void) -> Self {
        Self {
            native_window: window as usize as *mut c_void,
            native_display: display,
        }
    }

    #[inline]
    pub fn from_appkit(ns_view: *mut c_void) -> Self {
        Self {
            native_window: ns_view,
            native_display: std::ptr::null_mut(),
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        !self.native_window.is_null()
    }

    #[inline]
    pub fn xlib_window(&self) -> u64 {
        self.native_window as usize as u64
    }

    #[inline]
    pub fn xlib_display(&self) -> *mut c_void {
        self.native_display
    }

    #[inline]
    pub fn win32_hwnd(&self) -> *mut c_void {
        self.native_window
    }

    #[inline]
    pub fn appkit_view(&self) -> *mut c_void {
        self.native_window
    }
}

/// Configuration passed during pipeline initialization.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DuctInitConfig {
    pub window: DuctWindowHandle,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
    pub debug_mode: bool,
    pub app_name: *const c_char,
}

/// Shader stage identifier.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctShaderStage {
    Vertex = 1,
    Fragment = 2,
    Compute = 3,
}

/// Description used to compile or register a shader.
#[repr(C)]
pub struct DuctShaderDesc {
    pub stage: DuctShaderStage,
    pub source_bytes: *const u8,
    pub source_len: usize,
    pub entry_point: *const c_char,
}

/// Opaque shader handle.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DuctShaderHandle(pub u64);

impl DuctShaderHandle {
    pub const INVALID: Self = Self(0);

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Buffer type.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctBufferType {
    Vertex = 1,
    Index = 2,
    Uniform = 3,
    Storage = 4,
}

/// Description used to create a GPU buffer.
#[repr(C)]
pub struct DuctBufferDesc {
    pub buffer_type: DuctBufferType,
    pub size_bytes: usize,
    pub initial_data: *const u8,
    pub is_dynamic: bool,
}

/// Opaque buffer handle.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DuctBufferHandle(pub u64);

impl DuctBufferHandle {
    pub const INVALID: Self = Self(0);

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Texture format.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctTextureFormat {
    Rgba8 = 1,
    Rgb8 = 2,
    R8 = 3,
}

/// Opaque texture handle.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DuctTextureHandle(pub u64);

impl DuctTextureHandle {
    pub const INVALID: Self = Self(0);

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Face culling mode.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DuctCullMode {
    None = 0,
    Front = 1,
    Back = 2,
}

/// Blending mode.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DuctBlendMode {
    Opaque = 0,
    AlphaBlend = 1,
    Additive = 2,
}

/// Vertex data format for shader attribute inputs.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctVertexFormat {
    Float = 1,
    Float2 = 2,
    Float3 = 3,
    Float4 = 4,
    UByte4Normalized = 5,
}

impl DuctVertexFormat {
    pub fn size_bytes(self) -> u32 {
        match self {
            DuctVertexFormat::Float => 4,
            DuctVertexFormat::Float2 => 8,
            DuctVertexFormat::Float3 => 12,
            DuctVertexFormat::Float4 => 16,
            DuctVertexFormat::UByte4Normalized => 4,
        }
    }

    pub fn component_count(self) -> i32 {
        match self {
            DuctVertexFormat::Float => 1,
            DuctVertexFormat::Float2 => 2,
            DuctVertexFormat::Float3 => 3,
            DuctVertexFormat::Float4 => 4,
            DuctVertexFormat::UByte4Normalized => 4,
        }
    }
}

/// Description of a single vertex buffer input attribute.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DuctVertexAttribute {
    pub location: u32,
    pub format: DuctVertexFormat,
    pub offset: u32,
}

/// Layout descriptor for vertex attributes in a vertex buffer.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DuctVertexLayout {
    pub attributes: [DuctVertexAttribute; 8],
    pub attribute_count: u32,
    pub stride: u32,
}

impl Default for DuctVertexLayout {
    fn default() -> Self {
        Self {
            attributes: [DuctVertexAttribute {
                location: 0,
                format: DuctVertexFormat::Float3,
                offset: 0,
            }; 8],
            attribute_count: 0,
            stride: 0,
        }
    }
}

impl DuctVertexLayout {
    pub fn new(stride: u32) -> Self {
        Self {
            attributes: [DuctVertexAttribute {
                location: 0,
                format: DuctVertexFormat::Float3,
                offset: 0,
            }; 8],
            attribute_count: 0,
            stride,
        }
    }

    pub fn add_attribute(mut self, location: u32, format: DuctVertexFormat, offset: u32) -> Self {
        if (self.attribute_count as usize) < self.attributes.len() {
            self.attributes[self.attribute_count as usize] = DuctVertexAttribute {
                location,
                format,
                offset,
            };
            self.attribute_count += 1;
        }
        self
    }
}

/// Format of index buffer elements.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctIndexFormat {
    U16 = 1,
    U32 = 2,
}

/// Texture filtering mode.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctFilterMode {
    Nearest = 1,
    Linear = 2,
    LinearMipmapLinear = 3,
}

/// Texture coordinate wrapping mode.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctWrapMode {
    Repeat = 1,
    ClampToEdge = 2,
    MirroredRepeat = 3,
}

/// Primitive drawing topology.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DuctPrimitiveTopology {
    Triangles = 0,
    Lines = 1,
    LineStrip = 2,
    Points = 3,
    TriangleStrip = 4,
}

/// Depth and stencil comparison operators.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum DuctCompareOp {
    Always = 0,
    Never = 1,
    Less = 2,
    LessEqual = 3,
    Equal = 4,
    GreaterEqual = 5,
    Greater = 6,
    NotEqual = 7,
}

/// Opaque handle to an offscreen render target / framebuffer (FBO).
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DuctRenderTargetHandle(pub u64);

impl DuctRenderTargetHandle {
    pub const INVALID: Self = Self(0);

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

/// Description used to allocate an offscreen render target.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DuctRenderTargetDesc {
    pub width: u32,
    pub height: u32,
    pub format: DuctTextureFormat,
    pub has_depth: bool,
}

/// Real-time engine performance metrics and render statistics.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct DuctFrameStats {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub frame_count: u64,
    pub draw_calls: u32,
    pub triangles_rendered: u64,
    pub pipeline_switches: u32,
    pub allocated_gpu_memory_bytes: usize,
}

/// Full description of pipeline state.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DuctPipelineStateDesc {
    pub vertex_shader: DuctShaderHandle,
    pub fragment_shader: DuctShaderHandle,
    pub vertex_layout: DuctVertexLayout,
    pub cull_mode: DuctCullMode,
    pub blend_mode: DuctBlendMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub depth_compare: DuctCompareOp,
}

impl Default for DuctPipelineStateDesc {
    fn default() -> Self {
        Self {
            vertex_shader: DuctShaderHandle::INVALID,
            fragment_shader: DuctShaderHandle::INVALID,
            vertex_layout: DuctVertexLayout::default(),
            cull_mode: DuctCullMode::None,
            blend_mode: DuctBlendMode::AlphaBlend,
            depth_test: true,
            depth_write: true,
            depth_compare: DuctCompareOp::Less,
        }
    }
}

/// Low-level escape hatch information giving direct access to native GPU context / device.
#[repr(C)]
pub struct DuctRawContext {
    pub backend_type: DuctBackendType,
    /// Native context handle (HGLRC for OpenGL, ID3D12Device* for DX12, VkDevice for Vulkan)
    pub context_handle: *mut c_void,
    /// Secondary device handle (HDC for OpenGL, CommandQueue for DX12, VkQueue for Vulkan)
    pub device_handle: *mut c_void,
    /// Null-terminated string containing driver / API information
    pub extra_info: *const c_char,
    /// Optional function pointer to look up backend-specific procedure addresses (e.g., wglGetProcAddress)
    pub get_proc_address: Option<unsafe extern "C" fn(name: *const c_char) -> *mut c_void>,
}

/// The C-ABI vtable structure exported by every pipeline dynamic library.
#[repr(C)]
pub struct DuctPipelineVTable {
    /// Opaque pointer to backend instance state
    pub context: *mut c_void,
    /// Backend type identifier
    pub backend_type: DuctBackendType,
    /// Null-terminated UTF-8 backend name string (e.g. "OpenGL Core")
    pub name: *const c_char,
    /// Null-terminated UTF-8 backend version string (e.g. "4.5.0")
    pub version: *const c_char,

    // Lifecycle methods
    pub init: unsafe extern "C" fn(ctx: *mut c_void, config: *const DuctInitConfig) -> DuctResult,
    pub shutdown: unsafe extern "C" fn(ctx: *mut c_void) -> DuctResult,

    // Window surface & viewport
    pub resize_viewport: unsafe extern "C" fn(ctx: *mut c_void, width: u32, height: u32) -> DuctResult,
    pub set_scissor: unsafe extern "C" fn(ctx: *mut c_void, x: u32, y: u32, width: u32, height: u32) -> DuctResult,

    // Frame execution
    pub begin_frame: unsafe extern "C" fn(ctx: *mut c_void, clear_color: DuctColor) -> DuctResult,
    pub end_frame: unsafe extern "C" fn(ctx: *mut c_void) -> DuctResult,
    pub present: unsafe extern "C" fn(ctx: *mut c_void) -> DuctResult,

    // Shaders & Buffers
    pub create_shader: unsafe extern "C" fn(ctx: *mut c_void, desc: *const DuctShaderDesc, out_handle: *mut DuctShaderHandle) -> DuctResult,
    pub reload_shader: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctShaderHandle, desc: *const DuctShaderDesc) -> DuctResult,
    pub destroy_shader: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctShaderHandle) -> DuctResult,
    pub create_buffer: unsafe extern "C" fn(ctx: *mut c_void, desc: *const DuctBufferDesc, out_handle: *mut DuctBufferHandle) -> DuctResult,
    pub update_buffer: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctBufferHandle, offset: usize, data: *const u8, size: usize) -> DuctResult,
    pub destroy_buffer: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctBufferHandle) -> DuctResult,
    pub bind_vertex_buffer: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctBufferHandle) -> DuctResult,
    pub bind_index_buffer: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult,
    pub bind_uniform_buffer: unsafe extern "C" fn(ctx: *mut c_void, slot: u32, handle: DuctBufferHandle) -> DuctResult,

    // Textures & Mipmaps
    pub create_texture_2d: unsafe extern "C" fn(ctx: *mut c_void, width: u32, height: u32, format: DuctTextureFormat, data: *const u8, out_handle: *mut DuctTextureHandle) -> DuctResult,
    pub update_texture_2d: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctTextureHandle, x: u32, y: u32, width: u32, height: u32, data: *const u8, data_len: usize) -> DuctResult,
    pub destroy_texture: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctTextureHandle) -> DuctResult,
    pub bind_texture: unsafe extern "C" fn(ctx: *mut c_void, slot: u32, handle: DuctTextureHandle) -> DuctResult,
    pub generate_mipmaps: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctTextureHandle) -> DuctResult,
    pub set_texture_filter: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctTextureHandle, min_filter: DuctFilterMode, mag_filter: DuctFilterMode) -> DuctResult,
    pub set_texture_wrap: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctTextureHandle, wrap_s: DuctWrapMode, wrap_t: DuctWrapMode) -> DuctResult,

    // Render Targets (Offscreen FBOs)
    pub create_render_target: unsafe extern "C" fn(ctx: *mut c_void, desc: *const DuctRenderTargetDesc, out_handle: *mut DuctRenderTargetHandle) -> DuctResult,
    pub destroy_render_target: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctRenderTargetHandle) -> DuctResult,
    pub bind_render_target: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctRenderTargetHandle) -> DuctResult,
    pub get_render_target_texture: unsafe extern "C" fn(ctx: *mut c_void, handle: DuctRenderTargetHandle, out_texture: *mut DuctTextureHandle) -> DuctResult,
    pub blit_render_target: unsafe extern "C" fn(ctx: *mut c_void, src: DuctRenderTargetHandle, dst: DuctRenderTargetHandle, src_w: u32, src_h: u32, dst_w: u32, dst_h: u32, filter: DuctFilterMode) -> DuctResult,
    pub read_pixels: unsafe extern "C" fn(ctx: *mut c_void, x: u32, y: u32, width: u32, height: u32, out_rgba: *mut u8, buf_len: usize) -> DuctResult,

    // Pipeline state & drawing
    pub set_pipeline_state: unsafe extern "C" fn(ctx: *mut c_void, desc: *const DuctPipelineStateDesc) -> DuctResult,
    pub set_wireframe: unsafe extern "C" fn(ctx: *mut c_void, enabled: bool) -> DuctResult,
    pub set_vsync: unsafe extern "C" fn(ctx: *mut c_void, enabled: bool) -> DuctResult,
    pub set_uniform_int: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char, val: i32) -> DuctResult,
    pub set_uniform_float: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char, val: f32) -> DuctResult,
    pub set_uniform_vec2: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char, x: f32, y: f32) -> DuctResult,
    pub set_uniform_vec3: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char, x: f32, y: f32, z: f32) -> DuctResult,
    pub set_uniform_vec4: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char, x: f32, y: f32, z: f32, w: f32) -> DuctResult,
    pub set_uniform_mat4: unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char, elements_16: *const f32) -> DuctResult,

    pub draw: unsafe extern "C" fn(ctx: *mut c_void, vertex_count: u32, first_vertex: u32) -> DuctResult,
    pub draw_indexed: unsafe extern "C" fn(ctx: *mut c_void, index_count: u32, first_index: u32) -> DuctResult,
    pub draw_instanced: unsafe extern "C" fn(ctx: *mut c_void, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32) -> DuctResult,
    pub draw_indexed_instanced: unsafe extern "C" fn(ctx: *mut c_void, index_count: u32, instance_count: u32, first_index: u32, index_format: DuctIndexFormat) -> DuctResult,
    pub dispatch_compute: unsafe extern "C" fn(ctx: *mut c_void, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult,

    // Performance Stats & Raw context escape hatch
    pub get_frame_stats: unsafe extern "C" fn(ctx: *mut c_void, out_stats: *mut DuctFrameStats) -> DuctResult,
    pub get_raw_context: unsafe extern "C" fn(ctx: *mut c_void, out_raw: *mut DuctRawContext) -> DuctResult,

    // Custom backend command dispatch
    pub execute_custom: unsafe extern "C" fn(ctx: *mut c_void, command_id: u32, data: *mut c_void) -> DuctResult,

    // Destructor for the vtable and context
    pub destroy: unsafe extern "C" fn(vtable: *mut DuctPipelineVTable),
}

/// Standard C entry point signature to create a pipeline instance.
pub type DuctCreatePipelineFn = unsafe extern "C" fn() -> *mut DuctPipelineVTable;

/// Standard C entry point signature to destroy a pipeline instance.
pub type DuctDestroyPipelineFn = unsafe extern "C" fn(vtable: *mut DuctPipelineVTable);

pub const DUCT_CREATE_SYMBOL: &[u8] = b"duct_create_pipeline\0";
pub const DUCT_DESTROY_SYMBOL: &[u8] = b"duct_destroy_pipeline\0";

/// Idiomatic Rust trait implemented by backend engines.
pub trait RenderPipeline: 'static {
    fn backend_type(&self) -> DuctBackendType;
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    /// Returns the NDC clip space convention of this pipeline backend.
    fn clip_space(&self) -> DuctClipSpace {
        self.backend_type().clip_space()
    }

    fn init(&mut self, config: &DuctInitConfig) -> DuctResult;
    fn shutdown(&mut self) -> DuctResult;

    fn resize_viewport(&mut self, width: u32, height: u32) -> DuctResult;
    fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult;

    fn begin_frame(&mut self, clear_color: DuctColor) -> DuctResult;
    fn end_frame(&mut self) -> DuctResult;
    fn present(&mut self) -> DuctResult;

    fn create_shader(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult>;
    fn reload_shader(&mut self, _handle: DuctShaderHandle, _desc: &DuctShaderDesc) -> DuctResult {
        DuctResult::ErrorUnsupported
    }
    fn destroy_shader(&mut self, handle: DuctShaderHandle) -> DuctResult;

    fn create_buffer(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult>;
    fn update_buffer(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult;
    fn destroy_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult;
    fn bind_vertex_buffer(&mut self, _handle: DuctBufferHandle) -> DuctResult {
        DuctResult::Success
    }
    fn bind_index_buffer(&mut self, _handle: DuctBufferHandle, _format: DuctIndexFormat) -> DuctResult {
        DuctResult::Success
    }
    fn bind_uniform_buffer(&mut self, _slot: u32, _handle: DuctBufferHandle) -> DuctResult {
        DuctResult::Success
    }

    fn create_texture_2d(&mut self, width: u32, height: u32, format: DuctTextureFormat, data: Option<&[u8]>) -> Result<DuctTextureHandle, DuctResult>;
    fn update_texture_2d(&mut self, _handle: DuctTextureHandle, _x: u32, _y: u32, _width: u32, _height: u32, _data: &[u8]) -> DuctResult {
        DuctResult::Success
    }
    fn destroy_texture(&mut self, handle: DuctTextureHandle) -> DuctResult;
    fn bind_texture(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult;
    fn generate_mipmaps(&mut self, _handle: DuctTextureHandle) -> DuctResult {
        DuctResult::Success
    }
    fn set_texture_filter(&mut self, _handle: DuctTextureHandle, _min: DuctFilterMode, _mag: DuctFilterMode) -> DuctResult {
        DuctResult::Success
    }
    fn set_texture_wrap(&mut self, _handle: DuctTextureHandle, _wrap_s: DuctWrapMode, _wrap_t: DuctWrapMode) -> DuctResult {
        DuctResult::Success
    }

    fn create_render_target(&mut self, _desc: &DuctRenderTargetDesc) -> Result<DuctRenderTargetHandle, DuctResult> {
        Err(DuctResult::ErrorUnsupported)
    }
    fn destroy_render_target(&mut self, _handle: DuctRenderTargetHandle) -> DuctResult {
        DuctResult::Success
    }
    fn bind_render_target(&mut self, _handle: DuctRenderTargetHandle) -> DuctResult {
        DuctResult::Success
    }
    fn get_render_target_texture(&mut self, _handle: DuctRenderTargetHandle) -> Result<DuctTextureHandle, DuctResult> {
        Err(DuctResult::ErrorUnsupported)
    }
    fn blit_render_target(
        &mut self,
        _src: DuctRenderTargetHandle,
        _dst: DuctRenderTargetHandle,
        _src_w: u32,
        _src_h: u32,
        _dst_w: u32,
        _dst_h: u32,
        _filter: DuctFilterMode,
    ) -> DuctResult {
        DuctResult::Success
    }
    fn read_pixels(&mut self, _x: u32, _y: u32, _width: u32, _height: u32, _out_rgba: &mut [u8]) -> DuctResult {
        DuctResult::ErrorUnsupported
    }

    fn set_pipeline_state(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult;
    fn set_wireframe(&mut self, enabled: bool) -> DuctResult;
    fn set_vsync(&mut self, _enabled: bool) -> DuctResult {
        DuctResult::Success
    }
    fn set_uniform_int(&mut self, _name: &str, _val: i32) -> DuctResult {
        DuctResult::Success
    }
    fn set_uniform_float(&mut self, name: &str, val: f32) -> DuctResult;
    fn set_uniform_vec2(&mut self, name: &str, x: f32, y: f32) -> DuctResult;
    fn set_uniform_vec3(&mut self, name: &str, x: f32, y: f32, z: f32) -> DuctResult;
    fn set_uniform_vec4(&mut self, name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult;
    fn set_uniform_mat4(&mut self, name: &str, elements: &[f32; 16]) -> DuctResult;

    fn draw(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult;
    fn draw_indexed(&mut self, index_count: u32, first_index: u32) -> DuctResult;
    fn draw_instanced(&mut self, _vertex_count: u32, _instance_count: u32, _first_vertex: u32, _first_instance: u32) -> DuctResult {
        DuctResult::ErrorUnsupported
    }
    fn draw_indexed_instanced(&mut self, _index_count: u32, _instance_count: u32, _first_index: u32, _index_format: DuctIndexFormat) -> DuctResult {
        DuctResult::ErrorUnsupported
    }
    fn dispatch_compute(&mut self, _groups_x: u32, _groups_y: u32, _groups_z: u32) -> DuctResult {
        DuctResult::ErrorUnsupported
    }

    fn get_frame_stats(&mut self) -> Result<DuctFrameStats, DuctResult> {
        Ok(DuctFrameStats::default())
    }

    fn get_raw_context(&mut self) -> Result<DuctRawContext, DuctResult>;
    fn execute_custom(&mut self, command_id: u32, data: *mut c_void) -> DuctResult;
}

struct PipelineWrapper<T: RenderPipeline> {
    pipeline: T,
    name_cstr: std::ffi::CString,
    version_cstr: std::ffi::CString,
}

impl<T: RenderPipeline> PipelineWrapper<T> {
    fn new(pipeline: T) -> Self {
        let name_cstr = std::ffi::CString::new(pipeline.name()).unwrap_or_default();
        let version_cstr = std::ffi::CString::new(pipeline.version()).unwrap_or_default();
        Self {
            pipeline,
            name_cstr,
            version_cstr,
        }
    }
}

unsafe extern "C" fn vtable_init<T: RenderPipeline>(
    ctx: *mut c_void,
    config: *const DuctInitConfig,
) -> DuctResult {
    if ctx.is_null() || config.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.init(&*config)
}

unsafe extern "C" fn vtable_shutdown<T: RenderPipeline>(ctx: *mut c_void) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.shutdown()
}

unsafe extern "C" fn vtable_resize_viewport<T: RenderPipeline>(
    ctx: *mut c_void,
    width: u32,
    height: u32,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.resize_viewport(width, height)
}

unsafe extern "C" fn vtable_set_scissor<T: RenderPipeline>(
    ctx: *mut c_void,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_scissor(x, y, width, height)
}

unsafe extern "C" fn vtable_begin_frame<T: RenderPipeline>(
    ctx: *mut c_void,
    clear_color: DuctColor,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.begin_frame(clear_color)
}

unsafe extern "C" fn vtable_end_frame<T: RenderPipeline>(ctx: *mut c_void) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.end_frame()
}

unsafe extern "C" fn vtable_present<T: RenderPipeline>(ctx: *mut c_void) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.present()
}

unsafe extern "C" fn vtable_create_shader<T: RenderPipeline>(
    ctx: *mut c_void,
    desc: *const DuctShaderDesc,
    out_handle: *mut DuctShaderHandle,
) -> DuctResult {
    if ctx.is_null() || desc.is_null() || out_handle.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    match wrapper.pipeline.create_shader(&*desc) {
        Ok(handle) => {
            *out_handle = handle;
            DuctResult::Success
        }
        Err(err) => err,
    }
}

unsafe extern "C" fn vtable_reload_shader<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctShaderHandle,
    desc: *const DuctShaderDesc,
) -> DuctResult {
    if ctx.is_null() || desc.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.reload_shader(handle, &*desc)
}

unsafe extern "C" fn vtable_destroy_shader<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctShaderHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.destroy_shader(handle)
}

unsafe extern "C" fn vtable_create_buffer<T: RenderPipeline>(
    ctx: *mut c_void,
    desc: *const DuctBufferDesc,
    out_handle: *mut DuctBufferHandle,
) -> DuctResult {
    if ctx.is_null() || desc.is_null() || out_handle.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    match wrapper.pipeline.create_buffer(&*desc) {
        Ok(handle) => {
            *out_handle = handle;
            DuctResult::Success
        }
        Err(err) => err,
    }
}

unsafe extern "C" fn vtable_update_buffer<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctBufferHandle,
    offset: usize,
    data: *const u8,
    size: usize,
) -> DuctResult {
    if ctx.is_null() || data.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let slice = std::slice::from_raw_parts(data, size);
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.update_buffer(handle, offset, slice)
}

unsafe extern "C" fn vtable_destroy_buffer<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctBufferHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.destroy_buffer(handle)
}

unsafe extern "C" fn vtable_create_texture_2d<T: RenderPipeline>(
    ctx: *mut c_void,
    width: u32,
    height: u32,
    format: DuctTextureFormat,
    data: *const u8,
    out_handle: *mut DuctTextureHandle,
) -> DuctResult {
    if ctx.is_null() || out_handle.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let bytes_per_pixel = match format {
        DuctTextureFormat::Rgba8 => 4,
        DuctTextureFormat::Rgb8 => 3,
        DuctTextureFormat::R8 => 1,
    };
    let data_slice = if !data.is_null() {
        Some(std::slice::from_raw_parts(data, (width * height * bytes_per_pixel) as usize))
    } else {
        None
    };

    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    match wrapper.pipeline.create_texture_2d(width, height, format, data_slice) {
        Ok(handle) => {
            *out_handle = handle;
            DuctResult::Success
        }
        Err(err) => err,
    }
}

unsafe extern "C" fn vtable_destroy_texture<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctTextureHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.destroy_texture(handle)
}

unsafe extern "C" fn vtable_bind_texture<T: RenderPipeline>(
    ctx: *mut c_void,
    slot: u32,
    handle: DuctTextureHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.bind_texture(slot, handle)
}

unsafe extern "C" fn vtable_generate_mipmaps<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctTextureHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.generate_mipmaps(handle)
}

unsafe extern "C" fn vtable_set_texture_filter<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctTextureHandle,
    min_filter: DuctFilterMode,
    mag_filter: DuctFilterMode,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_texture_filter(handle, min_filter, mag_filter)
}

unsafe extern "C" fn vtable_create_render_target<T: RenderPipeline>(
    ctx: *mut c_void,
    desc: *const DuctRenderTargetDesc,
    out_handle: *mut DuctRenderTargetHandle,
) -> DuctResult {
    if ctx.is_null() || desc.is_null() || out_handle.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    match wrapper.pipeline.create_render_target(&*desc) {
        Ok(handle) => {
            *out_handle = handle;
            DuctResult::Success
        }
        Err(err) => err,
    }
}

unsafe extern "C" fn vtable_destroy_render_target<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctRenderTargetHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.destroy_render_target(handle)
}

unsafe extern "C" fn vtable_bind_render_target<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctRenderTargetHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.bind_render_target(handle)
}

unsafe extern "C" fn vtable_get_render_target_texture<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctRenderTargetHandle,
    out_texture: *mut DuctTextureHandle,
) -> DuctResult {
    if ctx.is_null() || out_texture.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    match wrapper.pipeline.get_render_target_texture(handle) {
        Ok(t) => {
            *out_texture = t;
            DuctResult::Success
        }
        Err(err) => err,
    }
}

unsafe extern "C" fn vtable_set_pipeline_state<T: RenderPipeline>(
    ctx: *mut c_void,
    desc: *const DuctPipelineStateDesc,
) -> DuctResult {
    if ctx.is_null() || desc.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_pipeline_state(&*desc)
}

unsafe extern "C" fn vtable_set_wireframe<T: RenderPipeline>(
    ctx: *mut c_void,
    enabled: bool,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_wireframe(enabled)
}

unsafe extern "C" fn vtable_set_uniform_float<T: RenderPipeline>(
    ctx: *mut c_void,
    name: *const c_char,
    val: f32,
) -> DuctResult {
    if ctx.is_null() || name.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let name_str = match std::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_uniform_float(name_str, val)
}

unsafe extern "C" fn vtable_set_uniform_vec2<T: RenderPipeline>(
    ctx: *mut c_void,
    name: *const c_char,
    x: f32,
    y: f32,
) -> DuctResult {
    if ctx.is_null() || name.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let name_str = match std::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_uniform_vec2(name_str, x, y)
}

unsafe extern "C" fn vtable_set_uniform_vec3<T: RenderPipeline>(
    ctx: *mut c_void,
    name: *const c_char,
    x: f32,
    y: f32,
    z: f32,
) -> DuctResult {
    if ctx.is_null() || name.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let name_str = match std::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_uniform_vec3(name_str, x, y, z)
}

unsafe extern "C" fn vtable_set_uniform_vec4<T: RenderPipeline>(
    ctx: *mut c_void,
    name: *const c_char,
    x: f32,
    y: f32,
    z: f32,
    w: f32,
) -> DuctResult {
    if ctx.is_null() || name.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let name_str = match std::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_uniform_vec4(name_str, x, y, z, w)
}

unsafe extern "C" fn vtable_set_uniform_mat4<T: RenderPipeline>(
    ctx: *mut c_void,
    name: *const c_char,
    elements_16: *const f32,
) -> DuctResult {
    if ctx.is_null() || name.is_null() || elements_16.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let name_str = match std::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    let slice = std::slice::from_raw_parts(elements_16, 16);
    let mut arr = [0.0f32; 16];
    arr.copy_from_slice(slice);

    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_uniform_mat4(name_str, &arr)
}

unsafe extern "C" fn vtable_draw<T: RenderPipeline>(
    ctx: *mut c_void,
    vertex_count: u32,
    first_vertex: u32,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.draw(vertex_count, first_vertex)
}

unsafe extern "C" fn vtable_draw_indexed<T: RenderPipeline>(
    ctx: *mut c_void,
    index_count: u32,
    first_index: u32,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.draw_indexed(index_count, first_index)
}

unsafe extern "C" fn vtable_draw_instanced<T: RenderPipeline>(
    ctx: *mut c_void,
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.draw_instanced(vertex_count, instance_count, first_vertex, first_instance)
}

unsafe extern "C" fn vtable_draw_indexed_instanced<T: RenderPipeline>(
    ctx: *mut c_void,
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    index_format: DuctIndexFormat,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.draw_indexed_instanced(index_count, instance_count, first_index, index_format)
}

unsafe extern "C" fn vtable_bind_vertex_buffer<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctBufferHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.bind_vertex_buffer(handle)
}

unsafe extern "C" fn vtable_bind_index_buffer<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctBufferHandle,
    format: DuctIndexFormat,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.bind_index_buffer(handle, format)
}

unsafe extern "C" fn vtable_bind_uniform_buffer<T: RenderPipeline>(
    ctx: *mut c_void,
    slot: u32,
    handle: DuctBufferHandle,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.bind_uniform_buffer(slot, handle)
}

unsafe extern "C" fn vtable_update_texture_2d<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctTextureHandle,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    data: *const u8,
    data_len: usize,
) -> DuctResult {
    if ctx.is_null() || data.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let slice = std::slice::from_raw_parts(data, data_len);
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.update_texture_2d(handle, x, y, width, height, slice)
}

unsafe extern "C" fn vtable_set_texture_wrap<T: RenderPipeline>(
    ctx: *mut c_void,
    handle: DuctTextureHandle,
    wrap_s: DuctWrapMode,
    wrap_t: DuctWrapMode,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_texture_wrap(handle, wrap_s, wrap_t)
}

unsafe extern "C" fn vtable_blit_render_target<T: RenderPipeline>(
    ctx: *mut c_void,
    src: DuctRenderTargetHandle,
    dst: DuctRenderTargetHandle,
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    filter: DuctFilterMode,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.blit_render_target(src, dst, src_w, src_h, dst_w, dst_h, filter)
}

unsafe extern "C" fn vtable_read_pixels<T: RenderPipeline>(
    ctx: *mut c_void,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    out_rgba: *mut u8,
    buf_len: usize,
) -> DuctResult {
    if ctx.is_null() || out_rgba.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let slice = std::slice::from_raw_parts_mut(out_rgba, buf_len);
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.read_pixels(x, y, width, height, slice)
}

unsafe extern "C" fn vtable_set_vsync<T: RenderPipeline>(
    ctx: *mut c_void,
    enabled: bool,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_vsync(enabled)
}

unsafe extern "C" fn vtable_set_uniform_int<T: RenderPipeline>(
    ctx: *mut c_void,
    name: *const c_char,
    val: i32,
) -> DuctResult {
    if ctx.is_null() || name.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let name_str = match std::ffi::CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.set_uniform_int(name_str, val)
}

unsafe extern "C" fn vtable_dispatch_compute<T: RenderPipeline>(
    ctx: *mut c_void,
    groups_x: u32,
    groups_y: u32,
    groups_z: u32,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.dispatch_compute(groups_x, groups_y, groups_z)
}

unsafe extern "C" fn vtable_get_frame_stats<T: RenderPipeline>(
    ctx: *mut c_void,
    out_stats: *mut DuctFrameStats,
) -> DuctResult {
    if ctx.is_null() || out_stats.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    match wrapper.pipeline.get_frame_stats() {
        Ok(s) => {
            *out_stats = s;
            DuctResult::Success
        }
        Err(err) => err,
    }
}

unsafe extern "C" fn vtable_get_raw_context<T: RenderPipeline>(
    ctx: *mut c_void,
    out_raw: *mut DuctRawContext,
) -> DuctResult {
    if ctx.is_null() || out_raw.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    match wrapper.pipeline.get_raw_context() {
        Ok(raw) => {
            *out_raw = raw;
            DuctResult::Success
        }
        Err(err) => err,
    }
}

unsafe extern "C" fn vtable_execute_custom<T: RenderPipeline>(
    ctx: *mut c_void,
    command_id: u32,
    data: *mut c_void,
) -> DuctResult {
    if ctx.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let wrapper = &mut *(ctx as *mut PipelineWrapper<T>);
    wrapper.pipeline.execute_custom(command_id, data)
}

unsafe extern "C" fn vtable_destroy<T: RenderPipeline>(vtable: *mut DuctPipelineVTable) {
    if vtable.is_null() {
        return;
    }
    let ctx = (*vtable).context;
    if !ctx.is_null() {
        let _ = Box::from_raw(ctx as *mut PipelineWrapper<T>);
    }
    let _ = Box::from_raw(vtable);
}

/// Constructs a boxed `DuctPipelineVTable` from a concrete `RenderPipeline` implementation.
pub fn build_vtable_from_pipeline<T: RenderPipeline>(pipeline: T) -> *mut DuctPipelineVTable {
    let backend_type = pipeline.backend_type();
    let wrapper = Box::new(PipelineWrapper::new(pipeline));
    let name_ptr = wrapper.name_cstr.as_ptr();
    let version_ptr = wrapper.version_cstr.as_ptr();
    let wrapper_ptr = Box::into_raw(wrapper) as *mut c_void;

    let vtable = Box::new(DuctPipelineVTable {
        context: wrapper_ptr,
        backend_type,
        name: name_ptr,
        version: version_ptr,
        init: vtable_init::<T>,
        shutdown: vtable_shutdown::<T>,
        resize_viewport: vtable_resize_viewport::<T>,
        set_scissor: vtable_set_scissor::<T>,
        begin_frame: vtable_begin_frame::<T>,
        end_frame: vtable_end_frame::<T>,
        present: vtable_present::<T>,
        create_shader: vtable_create_shader::<T>,
        reload_shader: vtable_reload_shader::<T>,
        destroy_shader: vtable_destroy_shader::<T>,
        create_buffer: vtable_create_buffer::<T>,
        update_buffer: vtable_update_buffer::<T>,
        destroy_buffer: vtable_destroy_buffer::<T>,
        bind_vertex_buffer: vtable_bind_vertex_buffer::<T>,
        bind_index_buffer: vtable_bind_index_buffer::<T>,
        bind_uniform_buffer: vtable_bind_uniform_buffer::<T>,
        create_texture_2d: vtable_create_texture_2d::<T>,
        update_texture_2d: vtable_update_texture_2d::<T>,
        destroy_texture: vtable_destroy_texture::<T>,
        bind_texture: vtable_bind_texture::<T>,
        generate_mipmaps: vtable_generate_mipmaps::<T>,
        set_texture_filter: vtable_set_texture_filter::<T>,
        set_texture_wrap: vtable_set_texture_wrap::<T>,
        create_render_target: vtable_create_render_target::<T>,
        destroy_render_target: vtable_destroy_render_target::<T>,
        bind_render_target: vtable_bind_render_target::<T>,
        get_render_target_texture: vtable_get_render_target_texture::<T>,
        blit_render_target: vtable_blit_render_target::<T>,
        read_pixels: vtable_read_pixels::<T>,
        set_pipeline_state: vtable_set_pipeline_state::<T>,
        set_wireframe: vtable_set_wireframe::<T>,
        set_vsync: vtable_set_vsync::<T>,
        set_uniform_int: vtable_set_uniform_int::<T>,
        set_uniform_float: vtable_set_uniform_float::<T>,
        set_uniform_vec2: vtable_set_uniform_vec2::<T>,
        set_uniform_vec3: vtable_set_uniform_vec3::<T>,
        set_uniform_vec4: vtable_set_uniform_vec4::<T>,
        set_uniform_mat4: vtable_set_uniform_mat4::<T>,
        draw: vtable_draw::<T>,
        draw_indexed: vtable_draw_indexed::<T>,
        draw_instanced: vtable_draw_instanced::<T>,
        draw_indexed_instanced: vtable_draw_indexed_instanced::<T>,
        dispatch_compute: vtable_dispatch_compute::<T>,
        get_frame_stats: vtable_get_frame_stats::<T>,
        get_raw_context: vtable_get_raw_context::<T>,
        execute_custom: vtable_execute_custom::<T>,
        destroy: vtable_destroy::<T>,
    });

    Box::into_raw(vtable)
}

/// Macro to export entry points `duct_create_pipeline` and `duct_destroy_pipeline` for any pipeline type.
#[macro_export]
macro_rules! export_pipeline {
    ($pipeline_type:ty, $specific_name:ident) => {
        pub fn create_pipeline_vtable() -> *mut $crate::DuctPipelineVTable {
            let instance = <$pipeline_type>::default();
            $crate::build_vtable_from_pipeline(instance)
        }

        #[no_mangle]
        pub extern "C" fn $specific_name() -> *mut $crate::DuctPipelineVTable {
            create_pipeline_vtable()
        }

        #[cfg(not(feature = "bundled"))]
        #[no_mangle]
        pub extern "C" fn duct_create_pipeline() -> *mut $crate::DuctPipelineVTable {
            create_pipeline_vtable()
        }

        #[cfg(not(feature = "bundled"))]
        #[no_mangle]
        pub extern "C" fn duct_destroy_pipeline(vtable: *mut $crate::DuctPipelineVTable) {
            if !vtable.is_null() {
                unsafe {
                    let destroy_fn = (*vtable).destroy;
                    destroy_fn(vtable);
                }
            }
        }
    };
    ($pipeline_type:ty) => {
        $crate::export_pipeline!($pipeline_type, duct_create_pipeline);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockPipeline;

    impl RenderPipeline for MockPipeline {
        fn backend_type(&self) -> DuctBackendType {
            DuctBackendType::Custom
        }
        fn name(&self) -> &str {
            "Mock Backend"
        }
        fn version(&self) -> &str {
            "1.0.0"
        }
        fn init(&mut self, config: &DuctInitConfig) -> DuctResult {
            if !config.window.is_valid() {
                DuctResult::ErrorWindowRequired
            } else {
                DuctResult::Success
            }
        }
        fn shutdown(&mut self) -> DuctResult {
            DuctResult::Success
        }
        fn resize_viewport(&mut self, _w: u32, _h: u32) -> DuctResult {
            DuctResult::Success
        }
        fn set_scissor(&mut self, _x: u32, _y: u32, _w: u32, _h: u32) -> DuctResult {
            DuctResult::Success
        }
        fn begin_frame(&mut self, _clear_color: DuctColor) -> DuctResult {
            DuctResult::Success
        }
        fn end_frame(&mut self) -> DuctResult {
            DuctResult::Success
        }
        fn present(&mut self) -> DuctResult {
            DuctResult::Success
        }
        fn create_shader(&mut self, _desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
            Ok(DuctShaderHandle(42))
        }
        fn destroy_shader(&mut self, _handle: DuctShaderHandle) -> DuctResult {
            DuctResult::Success
        }
        fn create_buffer(&mut self, _desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
            Ok(DuctBufferHandle(99))
        }
        fn update_buffer(&mut self, _h: DuctBufferHandle, _off: usize, _d: &[u8]) -> DuctResult {
            DuctResult::Success
        }
        fn destroy_buffer(&mut self, _handle: DuctBufferHandle) -> DuctResult {
            DuctResult::Success
        }
        fn create_texture_2d(&mut self, _w: u32, _h: u32, _f: DuctTextureFormat, _d: Option<&[u8]>) -> Result<DuctTextureHandle, DuctResult> {
            Ok(DuctTextureHandle(123))
        }
        fn destroy_texture(&mut self, _h: DuctTextureHandle) -> DuctResult {
            DuctResult::Success
        }
        fn bind_texture(&mut self, _s: u32, _h: DuctTextureHandle) -> DuctResult {
            DuctResult::Success
        }
        fn set_pipeline_state(&mut self, _desc: &DuctPipelineStateDesc) -> DuctResult {
            DuctResult::Success
        }
        fn set_wireframe(&mut self, _enabled: bool) -> DuctResult {
            DuctResult::Success
        }
        fn set_uniform_float(&mut self, _n: &str, _v: f32) -> DuctResult {
            DuctResult::Success
        }
        fn set_uniform_vec2(&mut self, _n: &str, _x: f32, _y: f32) -> DuctResult {
            DuctResult::Success
        }
        fn set_uniform_vec3(&mut self, _n: &str, _x: f32, _y: f32, _z: f32) -> DuctResult {
            DuctResult::Success
        }
        fn set_uniform_vec4(&mut self, _n: &str, _x: f32, _y: f32, _z: f32, _w: f32) -> DuctResult {
            DuctResult::Success
        }
        fn set_uniform_mat4(&mut self, _n: &str, _m: &[f32; 16]) -> DuctResult {
            DuctResult::Success
        }
        fn draw(&mut self, _vertex_count: u32, _first_vertex: u32) -> DuctResult {
            DuctResult::Success
        }
        fn draw_indexed(&mut self, _index_count: u32, _first_index: u32) -> DuctResult {
            DuctResult::Success
        }
        fn get_raw_context(&mut self) -> Result<DuctRawContext, DuctResult> {
            Ok(DuctRawContext {
                backend_type: DuctBackendType::Custom,
                context_handle: std::ptr::null_mut(),
                device_handle: std::ptr::null_mut(),
                extra_info: std::ptr::null(),
                get_proc_address: None,
            })
        }
        fn execute_custom(&mut self, _command_id: u32, _data: *mut c_void) -> DuctResult {
            DuctResult::Success
        }
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(DuctColor::BLACK.r, 0.0);
        assert_eq!(DuctColor::BLACK.a, 1.0);
        assert_eq!(DuctColor::WHITE.r, 1.0);
        assert_eq!(DuctColor::RED.r, 1.0);
        assert_eq!(DuctColor::RED.g, 0.0);
        assert_eq!(DuctColor::GREEN.g, 1.0);
        assert_eq!(DuctColor::BLUE.b, 1.0);
        assert_eq!(DuctColor::TRANSPARENT.a, 0.0);

        let c = DuctColor::from_rgba_u8(255, 128, 0, 255);
        assert_eq!(c.to_rgba_u8()[0], 255);
        assert_eq!(c.to_rgba_u8()[3], 255);
        assert_eq!(c.to_array()[0], 1.0);

        let rgb = DuctColor::rgb(0.5, 0.5, 0.5);
        assert_eq!(rgb.a, 1.0);
    }

    #[test]
    fn test_handles() {
        assert!(!DuctShaderHandle::INVALID.is_valid());
        assert!(DuctShaderHandle(1).is_valid());
        assert!(!DuctBufferHandle::INVALID.is_valid());
        assert!(DuctBufferHandle(1).is_valid());
        assert!(!DuctTextureHandle::INVALID.is_valid());
        assert!(DuctTextureHandle(1).is_valid());

        // Platform window handles
        let win = DuctWindowHandle::from_win32(0x1000 as *mut c_void, 0x2000 as *mut c_void);
        assert!(win.is_valid());
        assert_eq!(win.win32_hwnd(), 0x1000 as *mut c_void);

        let x11 = DuctWindowHandle::from_xlib(0xABCD, 0x5000 as *mut c_void);
        assert!(x11.is_valid());
        assert_eq!(x11.xlib_window(), 0xABCD);
        assert_eq!(x11.xlib_display(), 0x5000 as *mut c_void);

        let mac = DuctWindowHandle::from_appkit(0x9999 as *mut c_void);
        assert!(mac.is_valid());
        assert_eq!(mac.appkit_view(), 0x9999 as *mut c_void);

        // Render target handles
        assert!(!DuctRenderTargetHandle::INVALID.is_valid());
        assert!(DuctRenderTargetHandle(55).is_valid());

        // Vertex layout builder
        let layout = DuctVertexLayout::new(32)
            .add_attribute(0, DuctVertexFormat::Float3, 0)
            .add_attribute(1, DuctVertexFormat::Float2, 12)
            .add_attribute(2, DuctVertexFormat::Float3, 20);
        assert_eq!(layout.attribute_count, 3);
        assert_eq!(layout.stride, 32);
        assert_eq!(layout.attributes[1].format.size_bytes(), 8);
        assert_eq!(layout.attributes[1].format.component_count(), 2);
    }

    #[test]
    fn test_window_required() {
        let mut mock = MockPipeline;
        let config_null = DuctInitConfig {
            window: DuctWindowHandle::null(),
            width: 800,
            height: 600,
            vsync: false,
            debug_mode: false,
            app_name: std::ptr::null(),
        };
        assert_eq!(mock.init(&config_null), DuctResult::ErrorWindowRequired);

        let config_valid = DuctInitConfig {
            window: DuctWindowHandle::from_raw(1 as *mut c_void, std::ptr::null_mut()),
            width: 800,
            height: 600,
            vsync: false,
            debug_mode: false,
            app_name: std::ptr::null(),
        };
        assert_eq!(mock.init(&config_valid), DuctResult::Success);
    }
}
