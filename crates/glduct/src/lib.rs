//! OpenGL backend pipeline for Project32 duct graphics subsystem.
//! Compiles into `duct.gl.dll` (or platform equivalent).
//!
//! Pure zero-dependency OpenGL implementation (no external crates).
//! Performs real GPU OpenGL rendering, pipeline state management,
//! shaders, buffers, textures, uniforms, and buffer presentation to native window.

mod gl;

use duct_core::*;
use gl::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};

#[cfg(target_os = "windows")]
mod win_wgl {
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
struct GlShaderRecord {
    gl_id: u32,
    _stage: DuctShaderStage,
}

/// Record of an allocated OpenGL buffer.
struct GlBufferRecord {
    gl_id: u32,
    gl_target: u32,
    _size_bytes: usize,
}

/// Record of an allocated OpenGL texture.
struct GlTextureRecord {
    gl_id: u32,
    _width: u32,
    _height: u32,
    format: DuctTextureFormat,
}

/// Record of an allocated OpenGL Render Target (FBO).
struct GlRenderTargetRecord {
    fbo_id: u32,
    color_texture_handle: DuctTextureHandle,
    depth_rbo: u32,
    width: u32,
    height: u32,
}

/// Reference implementation of OpenGL pipeline for duct (Zero Dependencies).
pub struct GlPipeline {
    name: String,
    version: String,
    extra_info: CString,
    initialized: bool,

    gl: Option<Gl>,

    viewport_width: u32,
    viewport_height: u32,

    frame_in_progress: bool,
    frame_count: u64,
    draw_calls_in_frame: u32,
    vertices_drawn_in_frame: u32,

    default_vao: u32,
    next_handle_id: u64,
    shaders: HashMap<DuctShaderHandle, GlShaderRecord>,
    buffers: HashMap<DuctBufferHandle, GlBufferRecord>,
    textures: HashMap<DuctTextureHandle, GlTextureRecord>,
    render_targets: HashMap<DuctRenderTargetHandle, GlRenderTargetRecord>,
    active_render_target: DuctRenderTargetHandle,

    // Pipeline state caching
    current_vertex_layout: DuctVertexLayout,
    current_depth_compare: DuctCompareOp,

    // Real-time telemetry & frame stats
    last_frame_instant: Option<std::time::Instant>,
    current_frame_time_ms: f32,
    smoothed_fps: f32,
    pipeline_switches_in_frame: u32,
    allocated_buffer_bytes: usize,

    // Program cache: (vertex_shader_handle, fragment_shader_handle) -> gl_program_id
    programs: HashMap<(DuctShaderHandle, DuctShaderHandle), u32>,
    compute_programs: HashMap<DuctShaderHandle, u32>,
    active_program: u32,
    last_bound_vbo: u32,
    last_bound_ebo: u32,
    current_index_format: DuctIndexFormat,

    // Platform handles
    native_window: *mut c_void,
    native_display: *mut c_void,
    hdc: *mut c_void,
    hglrc: *mut c_void,
    owns_context: bool,
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
            native_window: std::ptr::null_mut(),
            native_display: std::ptr::null_mut(),
            hdc: std::ptr::null_mut(),
            hglrc: std::ptr::null_mut(),
            owns_context: false,
        }
    }
}

unsafe extern "C" fn gl_proc_loader(name: *const c_char) -> *mut c_void {
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
        std::ptr::null_mut()
    }
}

impl GlPipeline {
    #[inline]
    fn gl(&self) -> &Gl {
        self.gl.as_ref().expect("OpenGL function table not loaded")
    }

    fn apply_vertex_layout_and_vbo(&self, gl: &Gl) {
        unsafe {
            if self.default_vao != 0 {
                (gl.BindVertexArray)(self.default_vao);
            }
            if self.last_bound_vbo != 0 {
                (gl.BindBuffer)(GL_ARRAY_BUFFER, self.last_bound_vbo);
                if self.current_vertex_layout.attribute_count > 0 {
                    let stride = self.current_vertex_layout.stride as i32;
                    let count = (self.current_vertex_layout.attribute_count as usize)
                        .min(self.current_vertex_layout.attributes.len());
                    for attr in &self.current_vertex_layout.attributes[..count] {
                        let (size, gl_type, normalized) = match attr.format {
                            DuctVertexFormat::Float => (1, GL_FLOAT, GL_FALSE),
                            DuctVertexFormat::Float2 => (2, GL_FLOAT, GL_FALSE),
                            DuctVertexFormat::Float3 => (3, GL_FLOAT, GL_FALSE),
                            DuctVertexFormat::Float4 => (4, GL_FLOAT, GL_FALSE),
                            DuctVertexFormat::UByte4Normalized => (4, GL_UNSIGNED_BYTE, GL_TRUE),
                        };
                        (gl.VertexAttribPointer)(
                            attr.location,
                            size,
                            gl_type,
                            normalized,
                            stride,
                            attr.offset as usize as *const c_void,
                        );
                        (gl.EnableVertexAttribArray)(attr.location);
                    }
                } else {
                    // Fallback to location 0, 3 floats
                    (gl.VertexAttribPointer)(
                        0,
                        3,
                        GL_FLOAT,
                        GL_FALSE,
                        3 * std::mem::size_of::<f32>() as i32,
                        std::ptr::null(),
                    );
                    (gl.EnableVertexAttribArray)(0);
                }
            }
            if self.last_bound_ebo != 0 {
                (gl.BindBuffer)(GL_ELEMENT_ARRAY_BUFFER, self.last_bound_ebo);
            }
        }
    }
}

fn map_compare_op(op: DuctCompareOp) -> u32 {
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

fn map_filter_mode(filter: DuctFilterMode, is_min: bool) -> i32 {
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

fn map_wrap_mode(wrap: DuctWrapMode) -> i32 {
    match wrap {
        DuctWrapMode::Repeat => GL_REPEAT as i32,
        DuctWrapMode::ClampToEdge => GL_CLAMP_TO_EDGE,
        DuctWrapMode::MirroredRepeat => GL_MIRRORED_REPEAT,
    }
}

impl RenderPipeline for GlPipeline {
    fn backend_type(&self) -> DuctBackendType {
        DuctBackendType::OpenGL
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn init(&mut self, config: &DuctInitConfig) -> DuctResult {
        // Enforce: an attached native window is strictly required!
        if !config.window.is_valid() {
            eprintln!("[glduct][ERROR] Pipeline initialization failed: an attached native window is required.");
            return DuctResult::ErrorWindowRequired;
        }

        self.native_window = config.window.native_window;
        self.native_display = config.window.native_display;
        self.viewport_width = config.width;
        self.viewport_height = config.height;

        #[cfg(target_os = "windows")]
        {
            use win_wgl::*;
            let hwnd = self.native_window;
            let hdc = unsafe { GetDC(hwnd) };
            if hdc.is_null() {
                eprintln!("[glduct][ERROR] Win32 GetDC failed for native window handle {:?}", hwnd);
                return DuctResult::ErrorInitializationFailed;
            }
            self.hdc = hdc;

            // Configure Win32 Pixel Format for OpenGL
            let pfd = PIXELFORMATDESCRIPTOR {
                n_size: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
                n_version: 1,
                dw_flags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
                i_pixel_type: PFD_TYPE_RGBA,
                c_color_bits: 32,
                c_red_bits: 0,
                c_red_shift: 0,
                c_green_bits: 0,
                c_green_shift: 0,
                c_blue_bits: 0,
                c_blue_shift: 0,
                c_alpha_bits: 8,
                c_alpha_shift: 0,
                c_accum_bits: 0,
                c_accum_red_bits: 0,
                c_accum_green_bits: 0,
                c_accum_blue_bits: 0,
                c_accum_alpha_bits: 0,
                c_depth_bits: 24,
                c_stencil_bits: 8,
                c_aux_buffers: 0,
                i_layer_type: PFD_MAIN_PLANE,
                b_reserved: 0,
                dw_layer_mask: 0,
                dw_visible_mask: 0,
                dw_damage_mask: 0,
            };

            let pixel_format = unsafe { ChoosePixelFormat(hdc, &pfd) };
            if pixel_format != 0 {
                unsafe { SetPixelFormat(hdc, pixel_format, &pfd) };
            }

            // Create or bind WGL context
            let current_ctx = unsafe { wglGetCurrentContext() };
            if current_ctx.is_null() {
                let hglrc = unsafe { wglCreateContext(hdc) };
                if hglrc.is_null() {
                    eprintln!("[glduct][ERROR] wglCreateContext failed.");
                    return DuctResult::ErrorInitializationFailed;
                }
                unsafe { wglMakeCurrent(hdc, hglrc) };
                self.hglrc = hglrc;
                self.owns_context = true;
            } else {
                self.hglrc = current_ctx;
                self.owns_context = false;
            }

            // Dynamically load all OpenGL function pointers via our zero-dependency Gl loader
            let gl = unsafe {
                Gl::load_with(|s| {
                    let c_str = CString::new(s).unwrap();
                    get_gl_proc_address(c_str.as_ptr())
                })
            };
            self.gl = Some(gl);
        }

        #[cfg(target_os = "linux")]
        {
            use linux_glx::*;
            let mut display = self.native_display;
            if display.is_null() {
                display = unsafe { XOpenDisplay(std::ptr::null()) };
                self.native_display = display;
            }
            if display.is_null() {
                eprintln!("[glduct][ERROR] Failed to open X11 Display. Is $DISPLAY set?");
                return DuctResult::ErrorInitializationFailed;
            }

            let current_ctx = unsafe { glXGetCurrentContext() };
            if current_ctx.is_null() {
                let screen = unsafe { XDefaultScreen(display) };
                let attribs: [std::ffi::c_int; 11] = [4, 5, 12, 24, 8, 8, 9, 8, 10, 8, 0];
                let visual = unsafe { glXChooseVisual(display, screen, attribs.as_ptr()) };
                if visual.is_null() {
                    eprintln!("[glduct][ERROR] glXChooseVisual failed on X11 display.");
                    return DuctResult::ErrorInitializationFailed;
                }

                let ctx = unsafe { glXCreateContext(display, visual, std::ptr::null_mut(), 1) };
                if ctx.is_null() {
                    eprintln!("[glduct][ERROR] glXCreateContext failed.");
                    return DuctResult::ErrorInitializationFailed;
                }

                let xid = self.native_window as usize as u64;
                let ok = unsafe { glXMakeCurrent(display, xid, ctx) };
                if ok == 0 {
                    eprintln!("[glduct][ERROR] glXMakeCurrent failed for XID 0x{:X}.", xid);
                    return DuctResult::ErrorInitializationFailed;
                }
                self.hglrc = ctx;
                self.owns_context = true;
            } else {
                self.hglrc = current_ctx;
                self.owns_context = false;
            }

            let gl = unsafe {
                Gl::load_with(|s| {
                    let c_str = CString::new(s).unwrap();
                    get_gl_proc_address(c_str.as_ptr())
                })
            };
            self.gl = Some(gl);
        }

        #[cfg(target_os = "macos")]
        {
            use mac_cgl::*;
            let current_ctx = unsafe { CGLGetCurrentContext() };
            if current_ctx.is_null() {
                let attribs: [u32; 13] = [
                    K_CGL_PFA_ALL_RENDERERS,
                    K_CGL_PFA_DOUBLE_BUFFER,
                    K_CGL_PFA_COLOR_SIZE, 32,
                    K_CGL_PFA_DEPTH_SIZE, 24,
                    K_CGL_PFA_OPENGL_PROFILE, K_CGL_OGLP_VERSION_3_2_CORE,
                    0, 0,
                ];
                let mut pix: CGLPixelFormatObj = std::ptr::null_mut();
                let mut npix: std::ffi::c_int = 0;
                let err = unsafe { CGLChoosePixelFormat(attribs.as_ptr(), &mut pix, &mut npix) };
                if err != 0 || pix.is_null() {
                    eprintln!("[glduct][ERROR] CGLChoosePixelFormat failed on macOS (code {}).", err);
                    return DuctResult::ErrorInitializationFailed;
                }

                let mut ctx: CGLContextObj = std::ptr::null_mut();
                let err_ctx = unsafe { CGLCreateContext(pix, std::ptr::null_mut(), &mut ctx) };
                unsafe { CGLDestroyPixelFormat(pix); }
                if err_ctx != 0 || ctx.is_null() {
                    eprintln!("[glduct][ERROR] CGLCreateContext failed on macOS (code {}).", err_ctx);
                    return DuctResult::ErrorInitializationFailed;
                }

                let set_cur = unsafe { CGLSetCurrentContext(ctx) };
                if set_cur != 0 {
                    eprintln!("[glduct][ERROR] CGLSetCurrentContext failed on macOS (code {}).", set_cur);
                    return DuctResult::ErrorInitializationFailed;
                }
                self.hglrc = ctx;
                self.owns_context = true;
            } else {
                self.hglrc = current_ctx;
                self.owns_context = false;
            }

            let gl = unsafe {
                Gl::load_with(|s| {
                    let c_str = CString::new(s).unwrap();
                    get_gl_proc_address(c_str.as_ptr())
                })
            };
            self.gl = Some(gl);
        }

        if self.gl.is_none() {
            eprintln!("[glduct][ERROR] OpenGL is not supported or failed to initialize on this platform.");
            return DuctResult::ErrorUnsupported;
        }

        let gl = *self.gl();

        // Query active GPU driver and version
        let gl_version_str = unsafe {
            let ptr = (gl.GetString)(GL_VERSION);
            if !ptr.is_null() {
                CStr::from_ptr(ptr as *const i8).to_string_lossy().into_owned()
            } else {
                "Unknown OpenGL".to_string()
            }
        };

        let gl_renderer_str = unsafe {
            let ptr = (gl.GetString)(GL_RENDERER);
            if !ptr.is_null() {
                CStr::from_ptr(ptr as *const i8).to_string_lossy().into_owned()
            } else {
                "Unknown GPU".to_string()
            }
        };

        println!(
            "[glduct] OpenGL Pipeline initialized successfully!\n         Driver  : {}\n         Renderer: {}\n         Window  : {:?}\n         Viewport: {}x{}",
            gl_version_str, gl_renderer_str, self.native_window, self.viewport_width, self.viewport_height
        );

        self.name = format!("OpenGL ({})", gl_renderer_str);
        self.version = gl_version_str.clone();
        self.extra_info = CString::new(format!("{} | {}", gl_version_str, gl_renderer_str)).unwrap();

        // Create default VAO (required for Core Profile rendering)
        unsafe {
            let mut vao = 0u32;
            (gl.GenVertexArrays)(1, &mut vao);
            (gl.BindVertexArray)(vao);
            self.default_vao = vao;

            (gl.Viewport)(0, 0, self.viewport_width as i32, self.viewport_height as i32);
            (gl.Enable)(GL_DEPTH_TEST);
            (gl.DepthFunc)(GL_LESS);
            (gl.Enable)(GL_BLEND);
            (gl.BlendFunc)(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
        }

        self.initialized = true;
        let _ = self.set_vsync(config.vsync);
        DuctResult::Success
    }

    fn shutdown(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::Success;
        }

        println!("[glduct] Shutting down OpenGL pipeline (Total Frames: {})...", self.frame_count);

        if let Some(gl) = &self.gl {
            unsafe {
                for (_, rt) in self.render_targets.drain() {
                    let mut fbo = rt.fbo_id;
                    (gl.DeleteFramebuffers)(1, &mut fbo);
                    if rt.depth_rbo != 0 {
                        let mut rbo = rt.depth_rbo;
                        (gl.DeleteRenderbuffers)(1, &mut rbo);
                    }
                }
                for (_, p) in self.programs.drain() {
                    (gl.DeleteProgram)(p);
                }
                for (_, p) in self.compute_programs.drain() {
                    (gl.DeleteProgram)(p);
                }
                for (_, s) in self.shaders.drain() {
                    (gl.DeleteShader)(s.gl_id);
                }
                for (_, b) in self.buffers.drain() {
                    let mut id = b.gl_id;
                    (gl.DeleteBuffers)(1, &mut id);
                }
                for (_, t) in self.textures.drain() {
                    let mut id = t.gl_id;
                    (gl.DeleteTextures)(1, &mut id);
                }
                if self.default_vao != 0 {
                    (gl.DeleteVertexArrays)(1, &self.default_vao);
                    self.default_vao = 0;
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use win_wgl::*;
            if self.owns_context && !self.hglrc.is_null() {
                unsafe {
                    wglMakeCurrent(std::ptr::null_mut(), std::ptr::null_mut());
                    wglDeleteContext(self.hglrc);
                }
            }
            if !self.hdc.is_null() && !self.native_window.is_null() {
                unsafe {
                    ReleaseDC(self.native_window, self.hdc);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            use linux_glx::*;
            if self.owns_context && !self.hglrc.is_null() && !self.native_display.is_null() {
                unsafe {
                    glXMakeCurrent(self.native_display, 0, std::ptr::null_mut());
                    glXDestroyContext(self.native_display, self.hglrc);
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            use mac_cgl::*;
            if self.owns_context && !self.hglrc.is_null() {
                unsafe {
                    CGLSetCurrentContext(std::ptr::null_mut());
                    CGLDestroyContext(self.hglrc);
                }
            }
        }

        self.initialized = false;
        self.hglrc = std::ptr::null_mut();
        self.hdc = std::ptr::null_mut();
        DuctResult::Success
    }

    fn resize_viewport(&mut self, width: u32, height: u32) -> DuctResult {
        self.viewport_width = width;
        self.viewport_height = height;
        if let Some(gl) = &self.gl {
            unsafe {
                (gl.Viewport)(0, 0, width as i32, height as i32);
            }
        }
        DuctResult::Success
    }

    fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
        if let Some(gl) = &self.gl {
            unsafe {
                (gl.Enable)(GL_SCISSOR_TEST);
                (gl.Scissor)(x as i32, y as i32, width as i32, height as i32);
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

        if let Some(gl) = &self.gl {
            unsafe {
                (gl.ClearColor)(clear_color.r, clear_color.g, clear_color.b, clear_color.a);
                (gl.Clear)(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
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
        DuctResult::Success
    }

    fn present(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }

        #[cfg(target_os = "windows")]
        if !self.hdc.is_null() {
            unsafe {
                win_wgl::SwapBuffers(self.hdc);
            }
        }

        #[cfg(target_os = "linux")]
        if !self.native_display.is_null() && !self.native_window.is_null() {
            unsafe {
                linux_glx::glXSwapBuffers(self.native_display, self.native_window as usize as u64);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let ctx = if !self.hglrc.is_null() {
                self.hglrc
            } else {
                unsafe { mac_cgl::CGLGetCurrentContext() }
            };
            if !ctx.is_null() {
                unsafe {
                    mac_cgl::CGLFlushDrawable(ctx);
                }
            }
        }

        DuctResult::Success
    }

    fn create_shader(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        let gl = match &self.gl {
            Some(g) => g,
            None => return Err(DuctResult::ErrorInvalidState),
        };

        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return Err(DuctResult::ErrorInvalidParameter);
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut source_bytes = slice.to_vec();
        if !source_bytes.ends_with(&[0]) {
            source_bytes.push(0);
        }

        let gl_stage = match desc.stage {
            DuctShaderStage::Vertex => GL_VERTEX_SHADER,
            DuctShaderStage::Fragment => GL_FRAGMENT_SHADER,
            DuctShaderStage::Compute => GL_COMPUTE_SHADER,
        };

        let gl_id = unsafe { (gl.CreateShader)(gl_stage) };
        if gl_id == 0 {
            return Err(DuctResult::ErrorInitializationFailed);
        }

        let ptr = source_bytes.as_ptr() as *const c_char;
        unsafe {
            (gl.ShaderSource)(gl_id, 1, &ptr, std::ptr::null());
            (gl.CompileShader)(gl_id);

            let mut success = 0i32;
            (gl.GetShaderiv)(gl_id, GL_COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut log_len = 0i32;
                (gl.GetShaderiv)(gl_id, GL_INFO_LOG_LENGTH, &mut log_len);
                let mut buffer = vec![0u8; log_len as usize];
                (gl.GetShaderInfoLog)(
                    gl_id,
                    log_len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut c_char,
                );
                eprintln!(
                    "[glduct][ERROR] Shader compile failure ({:?}):\n{}",
                    desc.stage,
                    String::from_utf8_lossy(&buffer)
                );
                (gl.DeleteShader)(gl_id);
                return Err(DuctResult::ErrorInvalidParameter);
            }
        }

        let handle = DuctShaderHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.shaders.insert(handle, GlShaderRecord {
            gl_id,
            _stage: desc.stage,
        });

        Ok(handle)
    }

    fn reload_shader(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        let existing = match self.shaders.get(&handle) {
            Some(s) => s,
            None => return DuctResult::ErrorResourceNotFound,
        };

        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return DuctResult::ErrorInvalidParameter;
        }

        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let mut source_bytes = slice.to_vec();
        if !source_bytes.ends_with(&[0]) {
            source_bytes.push(0);
        }

        let gl_stage = match desc.stage {
            DuctShaderStage::Vertex => GL_VERTEX_SHADER,
            DuctShaderStage::Fragment => GL_FRAGMENT_SHADER,
            DuctShaderStage::Compute => GL_COMPUTE_SHADER,
        };

        let new_gl_id = unsafe { (gl.CreateShader)(gl_stage) };
        if new_gl_id == 0 {
            return DuctResult::ErrorInitializationFailed;
        }

        let ptr = source_bytes.as_ptr() as *const c_char;
        unsafe {
            (gl.ShaderSource)(new_gl_id, 1, &ptr, std::ptr::null());
            (gl.CompileShader)(new_gl_id);

            let mut success = 0i32;
            (gl.GetShaderiv)(new_gl_id, GL_COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut log_len = 0i32;
                (gl.GetShaderiv)(new_gl_id, GL_INFO_LOG_LENGTH, &mut log_len);
                let mut buffer = vec![0u8; log_len as usize];
                (gl.GetShaderInfoLog)(
                    new_gl_id,
                    log_len,
                    std::ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut c_char,
                );
                eprintln!(
                    "[glduct][WARN] Shader reload failed for {:?}:\n{}",
                    handle,
                    String::from_utf8_lossy(&buffer)
                );
                (gl.DeleteShader)(new_gl_id);
                return DuctResult::ErrorInvalidParameter;
            }

            // Success: delete old shader
            (gl.DeleteShader)(existing.gl_id);
        }

        self.shaders.insert(handle, GlShaderRecord {
            gl_id: new_gl_id,
            _stage: desc.stage,
        });

        // Invalidate cached programs that use this shader so they are relinked
        self.programs.retain(|&(vs, fs), &mut prog| {
            if vs == handle || fs == handle {
                if let Some(gl) = &self.gl {
                    unsafe { (gl.DeleteProgram)(prog); }
                }
                false
            } else {
                true
            }
        });

        println!("[glduct] Shader {:?} reloaded successfully!", handle);
        DuctResult::Success
    }

    fn destroy_shader(&mut self, handle: DuctShaderHandle) -> DuctResult {
        if let Some(record) = self.shaders.remove(&handle) {
            if let Some(gl) = &self.gl {
                unsafe {
                    (gl.DeleteShader)(record.gl_id);
                }
            }
            self.programs.retain(|&(vs, fs), &mut prog| {
                if vs == handle || fs == handle {
                    if let Some(gl) = &self.gl {
                        unsafe { (gl.DeleteProgram)(prog); }
                    }
                    false
                } else {
                    true
                }
            });
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_buffer(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
        let gl = match &self.gl {
            Some(g) => g,
            None => return Err(DuctResult::ErrorInvalidState),
        };

        let gl_target = match desc.buffer_type {
            DuctBufferType::Vertex => GL_ARRAY_BUFFER,
            DuctBufferType::Index => GL_ELEMENT_ARRAY_BUFFER,
            DuctBufferType::Uniform => GL_UNIFORM_BUFFER,
            DuctBufferType::Storage => GL_SHADER_STORAGE_BUFFER,
        };

        let usage = if desc.is_dynamic { GL_DYNAMIC_DRAW } else { GL_STATIC_DRAW };

        let mut gl_id = 0u32;
        unsafe {
            (gl.GenBuffers)(1, &mut gl_id);
            (gl.BindBuffer)(gl_target, gl_id);
            let ptr = if !desc.initial_data.is_null() {
                desc.initial_data as *const c_void
            } else {
                std::ptr::null()
            };
            (gl.BufferData)(gl_target, desc.size_bytes as isize, ptr, usage);
        }

        if desc.buffer_type == DuctBufferType::Vertex {
            self.last_bound_vbo = gl_id;
        } else if desc.buffer_type == DuctBufferType::Index {
            self.last_bound_ebo = gl_id;
        }

        self.allocated_buffer_bytes += desc.size_bytes;

        let handle = DuctBufferHandle(self.next_handle_id);
        self.next_handle_id += 1;

        self.buffers.insert(handle, GlBufferRecord {
            gl_id,
            gl_target,
            _size_bytes: desc.size_bytes,
        });

        Ok(handle)
    }

    fn update_buffer(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        if let Some(buf) = self.buffers.get(&handle) {
            unsafe {
                (gl.BindBuffer)(buf.gl_target, buf.gl_id);
                (gl.BufferSubData)(
                    buf.gl_target,
                    offset as isize,
                    data.len() as isize,
                    data.as_ptr() as *const c_void,
                );
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if let Some(buf) = self.buffers.remove(&handle) {
            self.allocated_buffer_bytes = self.allocated_buffer_bytes.saturating_sub(buf._size_bytes);
            if let Some(gl) = &self.gl {
                let mut id = buf.gl_id;
                unsafe {
                    (gl.DeleteBuffers)(1, &mut id);
                }
            }
            if self.last_bound_vbo == buf.gl_id {
                self.last_bound_vbo = 0;
            }
            if self.last_bound_ebo == buf.gl_id {
                self.last_bound_ebo = 0;
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_vertex_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(buf) = self.buffers.get(&handle) {
            self.last_bound_vbo = buf.gl_id;
            unsafe {
                (gl.BindBuffer)(GL_ARRAY_BUFFER, buf.gl_id);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_index_buffer(&mut self, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(buf) = self.buffers.get(&handle) {
            self.last_bound_ebo = buf.gl_id;
            self.current_index_format = format;
            unsafe {
                if self.default_vao != 0 {
                    (gl.BindVertexArray)(self.default_vao);
                }
                (gl.BindBuffer)(GL_ELEMENT_ARRAY_BUFFER, buf.gl_id);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_uniform_buffer(&mut self, slot: u32, handle: DuctBufferHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(buf) = self.buffers.get(&handle) {
            unsafe {
                (gl.BindBufferBase)(buf.gl_target, slot, buf.gl_id);
            }
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
        data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctResult> {
        let gl = match &self.gl {
            Some(g) => g,
            None => return Err(DuctResult::ErrorInvalidState),
        };

        let (internal_fmt, gl_fmt) = match format {
            DuctTextureFormat::Rgba8 => (GL_RGBA8, GL_RGBA),
            DuctTextureFormat::Rgb8 => (GL_RGB8, GL_RGB),
            DuctTextureFormat::R8 => (GL_R8, GL_RED),
        };

        let mut gl_id = 0u32;
        unsafe {
            (gl.GenTextures)(1, &mut gl_id);
            (gl.BindTexture)(GL_TEXTURE_2D, gl_id);
            (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR as i32);
            (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR as i32);
            (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_REPEAT as i32);
            (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_REPEAT as i32);

            if format == DuctTextureFormat::R8 || format == DuctTextureFormat::Rgb8 {
                (gl.PixelStorei)(GL_UNPACK_ALIGNMENT, 1);
            }

            let ptr = data.map(|d| d.as_ptr() as *const c_void).unwrap_or(std::ptr::null());
            (gl.TexImage2D)(
                GL_TEXTURE_2D,
                0,
                internal_fmt as i32,
                width as i32,
                height as i32,
                0,
                gl_fmt,
                GL_UNSIGNED_BYTE,
                ptr,
            );
        }

        let handle = DuctTextureHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.textures.insert(handle, GlTextureRecord {
            gl_id,
            _width: width,
            _height: height,
            format,
        });

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
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(tex) = self.textures.get(&handle) {
            let gl_fmt = match tex.format {
                DuctTextureFormat::Rgba8 => GL_RGBA,
                DuctTextureFormat::Rgb8 => GL_RGB,
                DuctTextureFormat::R8 => GL_RED,
            };
            unsafe {
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
                (gl.TexSubImage2D)(
                    GL_TEXTURE_2D,
                    0,
                    x as i32,
                    y as i32,
                    width as i32,
                    height as i32,
                    gl_fmt,
                    GL_UNSIGNED_BYTE,
                    data.as_ptr() as *const c_void,
                );
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_texture(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if let Some(tex) = self.textures.remove(&handle) {
            if let Some(gl) = &self.gl {
                let mut id = tex.gl_id;
                unsafe {
                    (gl.DeleteTextures)(1, &mut id);
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_texture(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        if let Some(tex) = self.textures.get(&handle) {
            unsafe {
                (gl.ActiveTexture)(GL_TEXTURE0 + slot);
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn generate_mipmaps(&mut self, handle: DuctTextureHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(tex) = self.textures.get(&handle) {
            unsafe {
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
                (gl.GenerateMipmap)(GL_TEXTURE_2D);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR_MIPMAP_LINEAR as i32);
            }
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
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(tex) = self.textures.get(&handle) {
            let min_val = map_filter_mode(min, true);
            let mag_val = map_filter_mode(mag, false);
            unsafe {
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, min_val as i32);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, mag_val as i32);
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
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        if let Some(tex) = self.textures.get(&handle) {
            let ws = map_wrap_mode(wrap_s);
            let wt = map_wrap_mode(wrap_t);
            unsafe {
                (gl.BindTexture)(GL_TEXTURE_2D, tex.gl_id);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, ws);
                (gl.TexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, wt);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_render_target(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        let gl = match self.gl {
            Some(g) => g,
            None => return Err(DuctResult::ErrorInvalidState),
        };

        let color_tex_handle = self.create_texture_2d(desc.width, desc.height, desc.format, None)?;
        let tex_gl_id = self.textures.get(&color_tex_handle).unwrap().gl_id;

        let mut fbo_id = 0u32;
        let mut depth_rbo = 0u32;

        unsafe {
            (gl.GenFramebuffers)(1, &mut fbo_id);
            (gl.BindFramebuffer)(GL_FRAMEBUFFER, fbo_id);
            (gl.FramebufferTexture2D)(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                tex_gl_id,
                0,
            );

            if desc.has_depth {
                (gl.GenRenderbuffers)(1, &mut depth_rbo);
                (gl.BindRenderbuffer)(GL_RENDERBUFFER, depth_rbo);
                (gl.RenderbufferStorage)(
                    GL_RENDERBUFFER,
                    GL_DEPTH24_STENCIL8,
                    desc.width as i32,
                    desc.height as i32,
                );
                (gl.FramebufferRenderbuffer)(
                    GL_FRAMEBUFFER,
                    GL_DEPTH_STENCIL_ATTACHMENT,
                    GL_RENDERBUFFER,
                    depth_rbo,
                );
            }

            let status = (gl.CheckFramebufferStatus)(GL_FRAMEBUFFER);
            if status != GL_FRAMEBUFFER_COMPLETE {
                eprintln!("[glduct][ERROR] Framebuffer incomplete: 0x{:X}", status);
                (gl.BindFramebuffer)(GL_FRAMEBUFFER, 0);
                (gl.DeleteFramebuffers)(1, &mut fbo_id);
                if depth_rbo != 0 {
                    (gl.DeleteRenderbuffers)(1, &mut depth_rbo);
                }
                let _ = self.destroy_texture(color_tex_handle);
                return Err(DuctResult::ErrorInitializationFailed);
            }

            // Restore previous or default framebuffer
            let prev_fbo = if self.active_render_target.is_valid() {
                self.render_targets
                    .get(&self.active_render_target)
                    .map(|rt| rt.fbo_id)
                    .unwrap_or(0)
            } else {
                0
            };
            (gl.BindFramebuffer)(GL_FRAMEBUFFER, prev_fbo);
        }

        let handle = DuctRenderTargetHandle(self.next_handle_id);
        self.next_handle_id += 1;

        self.render_targets.insert(
            handle,
            GlRenderTargetRecord {
                fbo_id,
                color_texture_handle: color_tex_handle,
                depth_rbo,
                width: desc.width,
                height: desc.height,
            },
        );

        Ok(handle)
    }

    fn destroy_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if let Some(rt) = self.render_targets.remove(&handle) {
            if self.active_render_target == handle {
                if let Some(gl) = &self.gl {
                    unsafe {
                        (gl.BindFramebuffer)(GL_FRAMEBUFFER, 0);
                        (gl.Viewport)(0, 0, self.viewport_width as i32, self.viewport_height as i32);
                    }
                }
                self.active_render_target = DuctRenderTargetHandle::INVALID;
            }

            if let Some(gl) = &self.gl {
                unsafe {
                    let mut fbo = rt.fbo_id;
                    (gl.DeleteFramebuffers)(1, &mut fbo);
                    if rt.depth_rbo != 0 {
                        let mut rbo = rt.depth_rbo;
                        (gl.DeleteRenderbuffers)(1, &mut rbo);
                    }
                }
            }
            let _ = self.destroy_texture(rt.color_texture_handle);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        if !handle.is_valid() {
            unsafe {
                (gl.BindFramebuffer)(GL_FRAMEBUFFER, 0);
                (gl.Viewport)(0, 0, self.viewport_width as i32, self.viewport_height as i32);
            }
            self.active_render_target = DuctRenderTargetHandle::INVALID;
            DuctResult::Success
        } else if let Some(rt) = self.render_targets.get(&handle) {
            let fbo = rt.fbo_id;
            let w = rt.width;
            let h = rt.height;
            unsafe {
                (gl.BindFramebuffer)(GL_FRAMEBUFFER, fbo);
                (gl.Viewport)(0, 0, w as i32, h as i32);
            }
            self.active_render_target = handle;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn get_render_target_texture(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        if let Some(rt) = self.render_targets.get(&handle) {
            Ok(rt.color_texture_handle)
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
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        let src_fbo = if src.is_valid() {
            match self.render_targets.get(&src) {
                Some(rt) => rt.fbo_id,
                None => return DuctResult::ErrorResourceNotFound,
            }
        } else {
            0
        };

        let dst_fbo = if dst.is_valid() {
            match self.render_targets.get(&dst) {
                Some(rt) => rt.fbo_id,
                None => return DuctResult::ErrorResourceNotFound,
            }
        } else {
            0
        };

        let gl_filter = match filter {
            DuctFilterMode::Nearest => GL_NEAREST as u32,
            _ => GL_LINEAR,
        };

        unsafe {
            (gl.BindFramebuffer)(GL_READ_FRAMEBUFFER, src_fbo);
            (gl.BindFramebuffer)(GL_DRAW_FRAMEBUFFER, dst_fbo);
            (gl.BlitFramebuffer)(
                0,
                0,
                src_w as i32,
                src_h as i32,
                0,
                0,
                dst_w as i32,
                dst_h as i32,
                GL_COLOR_BUFFER_BIT,
                gl_filter,
            );
            // Restore active render target
            let active_fbo = if self.active_render_target.is_valid() {
                self.render_targets
                    .get(&self.active_render_target)
                    .map(|rt| rt.fbo_id)
                    .unwrap_or(0)
            } else {
                0
            };
            (gl.BindFramebuffer)(GL_FRAMEBUFFER, active_fbo);
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
        let required = (width as usize) * (height as usize) * 4;
        if out_rgba.len() < required {
            return DuctResult::ErrorInvalidParameter;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        unsafe {
            (gl.PixelStorei)(GL_PACK_ALIGNMENT, 1);
            (gl.ReadPixels)(
                x as i32,
                y as i32,
                width as i32,
                height as i32,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                out_rgba.as_mut_ptr() as *mut c_void,
            );
        }
        DuctResult::Success
    }

    fn set_pipeline_state(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };

        let vs_rec = match self.shaders.get(&desc.vertex_shader) {
            Some(s) => s,
            None => return DuctResult::ErrorResourceNotFound,
        };

        // Support binding a standalone Compute Shader via vertex_shader slot when fragment_shader is INVALID
        if vs_rec._stage == DuctShaderStage::Compute && !desc.fragment_shader.is_valid() {
            let cs_id = vs_rec.gl_id;
            let program = if let Some(&p) = self.compute_programs.get(&desc.vertex_shader) {
                p
            } else {
                let prog = unsafe { (gl.CreateProgram)() };
                unsafe {
                    (gl.AttachShader)(prog, cs_id);
                    (gl.LinkProgram)(prog);
                    let mut ok = 0i32;
                    (gl.GetProgramiv)(prog, GL_LINK_STATUS, &mut ok);
                    if ok == 0 {
                        (gl.DeleteProgram)(prog);
                        return DuctResult::ErrorInitializationFailed;
                    }
                }
                self.compute_programs.insert(desc.vertex_shader, prog);
                prog
            };
            unsafe {
                (gl.UseProgram)(program);
            }
            self.active_program = program;
            self.pipeline_switches_in_frame += 1;
            return DuctResult::Success;
        }

        let vs = vs_rec.gl_id;
        let fs = match self.shaders.get(&desc.fragment_shader) {
            Some(s) => s.gl_id,
            None => return DuctResult::ErrorResourceNotFound,
        };

        self.current_vertex_layout = desc.vertex_layout;
        self.current_depth_compare = desc.depth_compare;
        self.pipeline_switches_in_frame += 1;

        let pair = (desc.vertex_shader, desc.fragment_shader);
        let program = if let Some(&p) = self.programs.get(&pair) {
            p
        } else {
            let prog = unsafe { (gl.CreateProgram)() };
            unsafe {
                (gl.AttachShader)(prog, vs);
                (gl.AttachShader)(prog, fs);
                (gl.LinkProgram)(prog);

                let mut ok = 0i32;
                (gl.GetProgramiv)(prog, GL_LINK_STATUS, &mut ok);
                if ok == 0 {
                    let mut log_len = 0i32;
                    (gl.GetProgramiv)(prog, GL_INFO_LOG_LENGTH, &mut log_len);
                    let mut buffer = vec![0u8; log_len as usize];
                    (gl.GetProgramInfoLog)(
                        prog,
                        log_len,
                        std::ptr::null_mut(),
                        buffer.as_mut_ptr() as *mut c_char,
                    );
                    eprintln!(
                        "[glduct][ERROR] Shader program link failure:\n{}",
                        String::from_utf8_lossy(&buffer)
                    );
                    (gl.DeleteProgram)(prog);
                    return DuctResult::ErrorInitializationFailed;
                }
            }
            self.programs.insert(pair, prog);
            prog
        };

        unsafe {
            (gl.UseProgram)(program);
            self.active_program = program;

            // Face Culling
            match desc.cull_mode {
                DuctCullMode::None => (gl.Disable)(GL_CULL_FACE),
                DuctCullMode::Front => {
                    (gl.Enable)(GL_CULL_FACE);
                    (gl.CullFace)(GL_FRONT);
                }
                DuctCullMode::Back => {
                    (gl.Enable)(GL_CULL_FACE);
                    (gl.CullFace)(GL_BACK);
                }
            }

            // Blending
            match desc.blend_mode {
                DuctBlendMode::Opaque => (gl.Disable)(GL_BLEND),
                DuctBlendMode::AlphaBlend => {
                    (gl.Enable)(GL_BLEND);
                    (gl.BlendFunc)(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
                }
                DuctBlendMode::Additive => {
                    (gl.Enable)(GL_BLEND);
                    (gl.BlendFunc)(GL_SRC_ALPHA, GL_ONE);
                }
            }

            // Depth
            if desc.depth_test {
                (gl.Enable)(GL_DEPTH_TEST);
            } else {
                (gl.Disable)(GL_DEPTH_TEST);
            }
            (gl.DepthMask)(if desc.depth_write { GL_TRUE } else { GL_FALSE });
            (gl.DepthFunc)(map_compare_op(desc.depth_compare));
        }

        DuctResult::Success
    }

    fn set_wireframe(&mut self, enabled: bool) -> DuctResult {
        if let Some(gl) = &self.gl {
            unsafe {
                (gl.PolygonMode)(GL_FRONT_AND_BACK, if enabled { GL_LINE } else { GL_FILL });
            }
        }
        DuctResult::Success
    }

    fn set_vsync(&mut self, enabled: bool) -> DuctResult {
        let interval: i32 = if enabled { 1 } else { 0 };
        #[cfg(target_os = "windows")]
        {
            let ptr = unsafe { win_wgl::get_gl_proc_address(b"wglSwapIntervalEXT\0".as_ptr() as *const i8) };
            if !ptr.is_null() {
                type WglSwapIntervalExtFn = unsafe extern "system" fn(i32) -> i32;
                let func: WglSwapIntervalExtFn = unsafe { std::mem::transmute(ptr) };
                unsafe { func(interval) };
            }
        }
        #[cfg(target_os = "linux")]
        {
            let ptr = unsafe { linux_glx::get_gl_proc_address(b"glXSwapIntervalMESA\0".as_ptr() as *const i8) };
            if !ptr.is_null() {
                type GlxSwapIntervalMesaFn = unsafe extern "C" fn(i32) -> i32;
                let func: GlxSwapIntervalMesaFn = unsafe { std::mem::transmute(ptr) };
                unsafe { func(interval) };
            }
        }
        let _ = interval;
        DuctResult::Success
    }

    fn set_uniform_int(&mut self, name: &str, val: i32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        let c_str = CString::new(name).unwrap_or_default();
        unsafe {
            let loc = (gl.GetUniformLocation)(self.active_program, c_str.as_ptr());
            if loc >= 0 {
                (gl.Uniform1i)(loc, val);
            }
        }
        DuctResult::Success
    }

    fn set_uniform_float(&mut self, name: &str, val: f32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        let c_str = CString::new(name).unwrap_or_default();
        unsafe {
            let loc = (gl.GetUniformLocation)(self.active_program, c_str.as_ptr());
            if loc >= 0 {
                (gl.Uniform1f)(loc, val);
            }
        }
        DuctResult::Success
    }

    fn set_uniform_vec2(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        let c_str = CString::new(name).unwrap_or_default();
        unsafe {
            let loc = (gl.GetUniformLocation)(self.active_program, c_str.as_ptr());
            if loc >= 0 {
                (gl.Uniform2f)(loc, x, y);
            }
        }
        DuctResult::Success
    }

    fn set_uniform_vec3(&mut self, name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        let c_str = CString::new(name).unwrap_or_default();
        unsafe {
            let loc = (gl.GetUniformLocation)(self.active_program, c_str.as_ptr());
            if loc >= 0 {
                (gl.Uniform3f)(loc, x, y, z);
            }
        }
        DuctResult::Success
    }

    fn set_uniform_vec4(&mut self, name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        let c_str = CString::new(name).unwrap_or_default();
        unsafe {
            let loc = (gl.GetUniformLocation)(self.active_program, c_str.as_ptr());
            if loc >= 0 {
                (gl.Uniform4f)(loc, x, y, z, w);
            }
        }
        DuctResult::Success
    }

    fn set_uniform_mat4(&mut self, name: &str, elements: &[f32; 16]) -> DuctResult {
        if self.active_program == 0 {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        let c_str = CString::new(name).unwrap_or_default();
        unsafe {
            let loc = (gl.GetUniformLocation)(self.active_program, c_str.as_ptr());
            if loc >= 0 {
                (gl.UniformMatrix4fv)(loc, 1, GL_FALSE, elements.as_ptr());
            }
        }
        DuctResult::Success
    }

    fn draw(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count;

        self.apply_vertex_layout_and_vbo(gl);
        unsafe {
            (gl.DrawArrays)(GL_TRIANGLES, first_vertex as i32, vertex_count as i32);
        }
        DuctResult::Success
    }

    fn draw_indexed(&mut self, index_count: u32, first_index: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count;

        self.apply_vertex_layout_and_vbo(gl);
        let (gl_idx_type, elem_size) = match self.current_index_format {
            DuctIndexFormat::U16 => (GL_UNSIGNED_SHORT, 2),
            DuctIndexFormat::U32 => (GL_UNSIGNED_INT, 4),
        };
        unsafe {
            let offset = (first_index * elem_size) as usize as *const c_void;
            (gl.DrawElements)(GL_TRIANGLES, index_count as i32, gl_idx_type, offset);
        }
        DuctResult::Success
    }

    fn draw_instanced(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        _first_instance: u32,
    ) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count * instance_count;

        self.apply_vertex_layout_and_vbo(gl);
        unsafe {
            (gl.DrawArraysInstanced)(
                GL_TRIANGLES,
                first_vertex as i32,
                vertex_count as i32,
                instance_count as i32,
            );
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
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count * instance_count;

        self.apply_vertex_layout_and_vbo(gl);
        let (gl_idx_type, elem_size) = match index_format {
            DuctIndexFormat::U16 => (GL_UNSIGNED_SHORT, 2),
            DuctIndexFormat::U32 => (GL_UNSIGNED_INT, 4),
        };
        unsafe {
            let offset = (first_index * elem_size) as usize as *const c_void;
            (gl.DrawElementsInstanced)(
                GL_TRIANGLES,
                index_count as i32,
                gl_idx_type,
                offset,
                instance_count as i32,
            );
        }
        DuctResult::Success
    }

    fn dispatch_compute(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        let gl = match &self.gl {
            Some(g) => g,
            None => return DuctResult::ErrorInvalidState,
        };
        unsafe {
            (gl.DispatchCompute)(groups_x.max(1), groups_y.max(1), groups_z.max(1));
            (gl.MemoryBarrier)(GL_ALL_BARRIER_BITS);
        }
        DuctResult::Success
    }

    fn get_frame_stats(&mut self) -> Result<DuctFrameStats, DuctResult> {
        Ok(DuctFrameStats {
            fps: self.smoothed_fps,
            frame_time_ms: self.current_frame_time_ms,
            frame_count: self.frame_count,
            draw_calls: self.draw_calls_in_frame,
            triangles_rendered: (self.vertices_drawn_in_frame / 3) as u64,
            pipeline_switches: self.pipeline_switches_in_frame,
            allocated_gpu_memory_bytes: self.allocated_buffer_bytes,
        })
    }

    fn get_raw_context(&mut self) -> Result<DuctRawContext, DuctResult> {
        let device_handle = if !self.hdc.is_null() {
            self.hdc
        } else {
            self.native_display
        };
        Ok(DuctRawContext {
            backend_type: DuctBackendType::OpenGL,
            context_handle: self.hglrc,
            device_handle,
            extra_info: self.extra_info.as_ptr(),
            get_proc_address: Some(gl_proc_loader),
        })
    }

    fn execute_custom(&mut self, command_id: u32, data: *mut c_void) -> DuctResult {
        match command_id {
            1 => {
                println!(
                    "[glduct] Custom Command 1: GL Status [Shaders: {}, Buffers: {}, Total Frames: {}]",
                    self.shaders.len(),
                    self.buffers.len(),
                    self.frame_count
                );
                DuctResult::Success
            }
            2 => {
                let bitmask = data as usize as u32;
                if let Some(gl) = &self.gl {
                    unsafe { (gl.Clear)(bitmask) };
                }
                DuctResult::Success
            }
            _ => {
                println!("[glduct] Custom Command ID {} executed with payload {:?}.", command_id, data);
                DuctResult::Success
            }
        }
    }
}

export_pipeline!(GlPipeline, glduct_create_pipeline);
