use crate::gl::*;
use crate::types::*;
use duct_core::*;
use std::ffi::{CStr, CString};

impl GlPipeline {
    #[inline]
    pub(crate) fn gl(&self) -> &Gl {
        self.gl.as_ref().expect("[glduct] OpenGL function table not initialized!")
    }
    pub(crate) fn init_impl(&mut self, config: &DuctInitConfig) -> DuctResult {
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

    pub(crate) fn shutdown_impl(&mut self) -> DuctResult {
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

        self.uniform_locations.clear();
        self.cached_uniform_cstrs.clear();

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

    pub(crate) fn resize_viewport_impl(&mut self, width: u32, height: u32) -> DuctResult {
        self.viewport_width = width;
        self.viewport_height = height;
        if let Some(gl) = &self.gl {
            unsafe {
                (gl.Viewport)(0, 0, width as i32, height as i32);
            }
        }
        DuctResult::Success
    }

    pub(crate) fn set_vsync_impl(&mut self, enabled: bool) -> DuctResult {
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

}
