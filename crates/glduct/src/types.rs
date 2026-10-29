use crate::gl::*;
use duct_core::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};

#[cfg(target_os = "windows")]
pub(crate) mod win_wgl {
    
    use std::ffi::c_void;

    #[repr(C)]
    pub struct PIXELFORMATDESCRIPTOR {
        pub n_size: u16,
        pub n_version: u16,
        pub dw_flags: u32,
        pub i_pixel_type: u8,
        pub c_color_bits: u8,
        pub c_red_bits: u8,
        pub c_red_shift: u8,
        pub c_green_bits: u8,
        pub c_green_shift: u8,
        pub c_blue_bits: u8,
        pub c_blue_shift: u8,
        pub c_alpha_bits: u8,
        pub c_alpha_shift: u8,
        pub c_accum_bits: u8,
        pub c_accum_red_bits: u8,
        pub c_accum_green_bits: u8,
        pub c_accum_blue_bits: u8,
        pub c_accum_alpha_bits: u8,
        pub c_depth_bits: u8,
        pub c_stencil_bits: u8,
        pub c_aux_buffers: u8,
        pub i_layer_type: u8,
        pub b_reserved: u8,
        pub dw_layer_mask: u32,
        pub dw_visible_mask: u32,
        pub dw_damage_mask: u32,
    }

    pub const PFD_DRAW_TO_WINDOW: u32 = 0x0000_0004;
    pub const PFD_SUPPORT_OPENGL: u32 = 0x0000_0020;
    pub const PFD_DOUBLEBUFFER: u32 = 0x0000_0001;
    pub const PFD_TYPE_RGBA: u8 = 0;
    pub const PFD_MAIN_PLANE: u8 = 0;

    #[link(name = "opengl32")]
    unsafe extern "system" {
        pub fn wglGetProcAddress(name: *const i8) -> *const c_void;
        pub fn wglGetCurrentContext() -> *mut c_void;
        pub fn wglCreateContext(hdc: *mut c_void) -> *mut c_void;
        pub fn wglMakeCurrent(hdc: *mut c_void, hglrc: *mut c_void) -> i32;
        pub fn wglDeleteContext(hglrc: *mut c_void) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn GetModuleHandleA(lpModuleName: *const i8) -> *mut c_void;
        pub fn GetProcAddress(hModule: *mut c_void, lpProcName: *const i8) -> *const c_void;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn GetDC(hwnd: *mut c_void) -> *mut c_void;
        pub fn ReleaseDC(hwnd: *mut c_void, hdc: *mut c_void) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        pub fn ChoosePixelFormat(hdc: *mut c_void, ppfd: *const PIXELFORMATDESCRIPTOR) -> i32;
        pub fn SetPixelFormat(hdc: *mut c_void, format: i32, ppfd: *const PIXELFORMATDESCRIPTOR) -> i32;
        pub fn SwapBuffers(hdc: *mut c_void) -> i32;
    }

    pub unsafe fn get_gl_proc_address(name: *const i8) -> *const c_void {
        let p = unsafe { wglGetProcAddress(name) };
        if !p.is_null() && p as isize != 1 && p as isize != 2 && p as isize != 3 && p as isize != -1 {
            return p;
        }
        let module = unsafe { GetModuleHandleA(b"opengl32.dll\0".as_ptr() as *const i8) };
        if module.is_null() {
            return std::ptr::null();
        }
        unsafe { GetProcAddress(module, name) }
    }
}

#[cfg(target_os = "linux")]
#[allow(non_snake_case, dead_code)]
mod linux_glx {
    use std::ffi::{c_char, c_int, c_void};

    #[link(name = "GL")]
    unsafe extern "C" {
        pub fn glXChooseVisual(dpy: *mut c_void, screen: c_int, attribList: *const c_int) -> *mut c_void;
        pub fn glXCreateContext(dpy: *mut c_void, vis: *mut c_void, shareList: *mut c_void, direct: c_int) -> *mut c_void;
        pub fn glXMakeCurrent(dpy: *mut c_void, drawable: u64, ctx: *mut c_void) -> c_int;
        pub fn glXSwapBuffers(dpy: *mut c_void, drawable: u64);
        pub fn glXDestroyContext(dpy: *mut c_void, ctx: *mut c_void);
        pub fn glXGetCurrentContext() -> *mut c_void;
        pub fn glXGetProcAddress(procName: *const u8) -> *const c_void;
        pub fn glXGetProcAddressARB(procName: *const u8) -> *const c_void;
    }

    #[link(name = "X11")]
    unsafe extern "C" {
        pub fn XOpenDisplay(display_name: *const c_char) -> *mut c_void;
        pub fn XCloseDisplay(display: *mut c_void) -> c_int;
        pub fn XDefaultScreen(display: *mut c_void) -> c_int;
    }

    #[link(name = "dl")]
    unsafe extern "C" {
        pub fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
        pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    pub unsafe fn get_gl_proc_address(name: *const i8) -> *const c_void {
        let p = unsafe { glXGetProcAddressARB(name as *const u8) };
        if !p.is_null() {
            return p;
        }
        let p2 = unsafe { glXGetProcAddress(name as *const u8) };
        if !p2.is_null() {
            return p2;
        }

        static GL_LIB: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        let handle = *GL_LIB.get_or_init(|| {
            let path = b"libGL.so.1\0";
            let mut ptr = unsafe { dlopen(path.as_ptr() as *const c_char, 2) };
            if ptr.is_null() {
                ptr = unsafe { dlopen(b"libGL.so\0".as_ptr() as *const c_char, 2) };
            }
            ptr as usize
        });

        if handle != 0 {
            let sym = unsafe { dlsym(handle as *mut c_void, name as *const c_char) };
            if !sym.is_null() {
                return sym;
            }
        }
        std::ptr::null()
    }
}

#[cfg(target_os = "macos")]
#[allow(non_snake_case, dead_code)]
mod mac_cgl {
    use std::ffi::{c_char, c_int, c_void};

    pub type CGLContextObj = *mut c_void;
    pub type CGLPixelFormatObj = *mut c_void;
    pub type CGLError = c_int;

    pub const K_CGL_PFA_ALL_RENDERERS: u32 = 1;
    pub const K_CGL_PFA_DOUBLE_BUFFER: u32 = 5;
    pub const K_CGL_PFA_COLOR_SIZE: u32 = 8;
    pub const K_CGL_PFA_DEPTH_SIZE: u32 = 12;
    pub const K_CGL_PFA_OPENGL_PROFILE: u32 = 99;
    pub const K_CGL_OGLP_VERSION_3_2_CORE: u32 = 0x3200;

    #[link(name = "OpenGL", kind = "framework")]
    unsafe extern "C" {
        pub fn CGLChoosePixelFormat(
            attribs: *const u32,
            pix: *mut CGLPixelFormatObj,
            npix: *mut c_int,
        ) -> CGLError;
        pub fn CGLDestroyPixelFormat(pix: CGLPixelFormatObj) -> CGLError;
        pub fn CGLCreateContext(
            pix: CGLPixelFormatObj,
            share: CGLContextObj,
            ctx: *mut CGLContextObj,
        ) -> CGLError;
        pub fn CGLDestroyContext(ctx: CGLContextObj) -> CGLError;
        pub fn CGLSetCurrentContext(ctx: CGLContextObj) -> CGLError;
        pub fn CGLGetCurrentContext() -> CGLContextObj;
        pub fn CGLFlushDrawable(ctx: CGLContextObj) -> CGLError;
    }

    unsafe extern "C" {
        pub fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
        pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    pub unsafe fn get_gl_proc_address(name: *const i8) -> *const c_void {
        static GL_FRAMEWORK: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        let handle = *GL_FRAMEWORK.get_or_init(|| {
            let path = b"/System/Library/Frameworks/OpenGL.framework/OpenGL\0";
            let ptr = unsafe { dlopen(path.as_ptr() as *const c_char, 2) };
            ptr as usize
        });

        if handle != 0 {
            let sym = unsafe { dlsym(handle as *mut c_void, name as *const c_char) };
            if !sym.is_null() {
                return sym;
            }
        }
        // RTLD_DEFAULT fallback on macOS is -2isize
        unsafe { dlsym(-2isize as *mut c_void, name as *const c_char) }
    }
}



/// Record of an allocated OpenGL shader.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlShaderRecord {
    pub(crate) gl_id: u32,
    pub(crate) _stage: DuctShaderStage,
}

/// Record of an allocated OpenGL buffer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlBufferRecord {
    pub(crate) gl_id: u32,
    pub(crate) gl_target: u32,
    pub(crate) _size_bytes: usize,
}

/// Record of an allocated OpenGL texture.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlTextureRecord {
    pub(crate) gl_id: u32,
    pub(crate) _width: u32,
    pub(crate) _height: u32,
    pub(crate) format: DuctTextureFormat,
}

/// Record of an allocated OpenGL Render Target (FBO).
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlRenderTargetRecord {
    pub(crate) fbo_id: u32,
    pub(crate) color_texture_handle: DuctTextureHandle,
    pub(crate) depth_texture_handle: DuctTextureHandle,
    pub(crate) depth_rbo: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}
pub struct GlPipeline {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) extra_info: CString,
    pub(crate) initialized: bool,

    pub(crate) gl: Option<Gl>,

    pub(crate) viewport_width: u32,
    pub(crate) viewport_height: u32,

    pub(crate) frame_in_progress: bool,
    pub(crate) frame_count: u64,
    pub(crate) draw_calls_in_frame: u32,
    pub(crate) vertices_drawn_in_frame: u32,

    pub(crate) default_vao: u32,
    pub(crate) next_handle_id: u64,
    pub(crate) shaders: HashMap<DuctShaderHandle, GlShaderRecord>,
    pub(crate) buffers: HashMap<DuctBufferHandle, GlBufferRecord>,
    pub(crate) textures: HashMap<DuctTextureHandle, GlTextureRecord>,
    pub(crate) render_targets: HashMap<DuctRenderTargetHandle, GlRenderTargetRecord>,
    pub(crate) active_render_target: DuctRenderTargetHandle,

    // Pipeline state caching
    pub(crate) current_vertex_layout: DuctVertexLayout,
    pub(crate) current_depth_compare: DuctCompareOp,

    // Real-time telemetry & frame stats
    pub(crate) last_frame_instant: Option<std::time::Instant>,
    pub(crate) current_frame_time_ms: f32,
    pub(crate) smoothed_fps: f32,
    pub(crate) pipeline_switches_in_frame: u32,
    pub(crate) allocated_buffer_bytes: usize,

    // Program cache: (vertex_shader_handle, fragment_shader_handle) -> gl_program_id
    pub(crate) programs: HashMap<(DuctShaderHandle, DuctShaderHandle), u32>,
    pub(crate) compute_programs: HashMap<DuctShaderHandle, u32>,
    pub(crate) active_program: u32,
    pub(crate) last_bound_vbo: u32,
    pub(crate) last_bound_ebo: u32,
    pub(crate) current_index_format: DuctIndexFormat,

    // Uniform location cache: gl_program_id -> HashMap<String, i32>
    pub(crate) uniform_locations: HashMap<u32, HashMap<String, i32>>,
    // Reusable cached CString uniform names (avoids per-call heap allocation)
    pub(crate) cached_uniform_cstrs: HashMap<String, CString>,

    // Platform handles
    pub(crate) native_window: *mut c_void,
    pub(crate) native_display: *mut c_void,
    pub(crate) hdc: *mut c_void,
    pub(crate) hglrc: *mut c_void,
    pub(crate) owns_context: bool,
}

impl Default for GlPipeline {
    fn default() -> Self {
        Self {
            name: "OpenGL Core Pipeline".to_string(),
            version: "0.1.0".to_string(),
            extra_info: CString::new("OpenGL 4.5.0 - Project32 Duct Real GL Engine").unwrap(),
            initialized: false,
            gl: None,
            viewport_width: 800,
            viewport_height: 600,
            frame_in_progress: false,
            frame_count: 0,
            draw_calls_in_frame: 0,
            vertices_drawn_in_frame: 0,
            default_vao: 0,
            next_handle_id: 1,
            shaders: HashMap::new(),
            buffers: HashMap::new(),
            textures: HashMap::new(),
            render_targets: HashMap::new(),
            active_render_target: DuctRenderTargetHandle::INVALID,
            current_vertex_layout: DuctVertexLayout::default(),
            current_depth_compare: DuctCompareOp::Less,
            last_frame_instant: None,
            current_frame_time_ms: 16.6,
            smoothed_fps: 60.0,
            pipeline_switches_in_frame: 0,
            allocated_buffer_bytes: 0,
            programs: HashMap::new(),
            compute_programs: HashMap::new(),
            active_program: 0,
            last_bound_vbo: 0,
            last_bound_ebo: 0,
            current_index_format: DuctIndexFormat::U32,
            uniform_locations: HashMap::new(),
            cached_uniform_cstrs: HashMap::new(),
            native_window: std::ptr::null_mut(),
            native_display: std::ptr::null_mut(),
            hdc: std::ptr::null_mut(),
            hglrc: std::ptr::null_mut(),
            owns_context: false,
        }
    }
}

pub(crate) unsafe extern "C" fn gl_proc_loader(name: *const c_char) -> *mut c_void {
    #[cfg(target_os = "windows")]
    {
        win_wgl::get_gl_proc_address(name as *const i8) as *mut c_void
    }
    #[cfg(target_os = "linux")]
    {
        linux_glx::get_gl_proc_address(name as *const i8) as *mut c_void
    }
    #[cfg(target_os = "macos")]
    {
        mac_cgl::get_gl_proc_address(name as *const i8) as *mut c_void
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        let _ = name;
    }
}


pub(crate) fn map_compare_op(op: DuctCompareOp) -> u32 {
    match op {
        DuctCompareOp::Always => GL_ALWAYS,
        DuctCompareOp::Never => GL_NEVER,
        DuctCompareOp::Less => GL_LESS,
        DuctCompareOp::LessEqual => GL_LEQUAL,
        DuctCompareOp::Equal => GL_EQUAL,
        DuctCompareOp::GreaterEqual => GL_GEQUAL,
        DuctCompareOp::Greater => GL_GREATER,
        DuctCompareOp::NotEqual => GL_NOTEQUAL,
    }
}

pub(crate) fn map_filter_mode(filter: DuctFilterMode, is_min: bool) -> i32 {
    match filter {
        DuctFilterMode::Nearest => GL_NEAREST,
        DuctFilterMode::Linear => GL_LINEAR as i32,
        DuctFilterMode::LinearMipmapLinear => {
            if is_min {
                GL_LINEAR_MIPMAP_LINEAR
            } else {
                GL_LINEAR as i32
            }
        }
    }
}

pub(crate) fn map_wrap_mode(wrap: DuctWrapMode) -> i32 {
    match wrap {
        DuctWrapMode::Repeat => GL_REPEAT as i32,
        DuctWrapMode::ClampToEdge => GL_CLAMP_TO_EDGE,
        DuctWrapMode::MirroredRepeat => GL_MIRRORED_REPEAT,
    }
}

