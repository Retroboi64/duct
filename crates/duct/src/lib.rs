//! Duct: Central router and dynamic graphics pipeline controller for Project32.
//!
//! Pure zero-dependency Rust implementation (no external crates besides optional windowed).
//! Uses raw OS dynamic library primitives (`LoadLibraryW` on Windows, `dlopen` on Unix).
//! Compiles into `duct.dll` (dynamic library with C-ABI exports for multi-language support)
//! and a Rust `rlib` for direct Rust consumption.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub use duct_core::*;

use std::ffi::{c_char, c_void, CStr, CString};
use std::fmt;
use std::path::{Path, PathBuf};

// ── Zero-Dependency OS Dynamic Library Loader ───────────────────────────────

#[cfg(target_os = "windows")]
mod os_lib {
    use std::ffi::c_void;
    use std::path::Path;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
        fn GetProcAddress(hModule: *mut c_void, lpProcName: *const i8) -> *const c_void;
        fn FreeLibrary(hLibModule: *mut c_void) -> i32;
    }

    pub struct OsLibrary {
        handle: *mut c_void,
    }

    unsafe impl Send for OsLibrary {}
    unsafe impl Sync for OsLibrary {}

    impl OsLibrary {
        pub fn load(path: &Path) -> Result<Self, String> {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
            if handle.is_null() {
                Err(format!("LoadLibraryW failed for {}", path.display()))
            } else {
                Ok(Self { handle })
            }
        }

        pub unsafe fn get_symbol<T>(&self, symbol: &[u8]) -> Result<T, String> {
            let proc = unsafe { GetProcAddress(self.handle, symbol.as_ptr() as *const i8) };
            if proc.is_null() {
                Err(format!(
                    "Symbol '{}' not found",
                    String::from_utf8_lossy(symbol)
                ))
            } else {
                Ok(std::mem::transmute_copy(&proc))
            }
        }
    }

    impl Drop for OsLibrary {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                unsafe {
                    FreeLibrary(self.handle);
                }
                self.handle = std::ptr::null_mut();
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod os_lib {
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::path::Path;

    #[cfg(target_os = "linux")]
    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> i32;
        fn dlerror() -> *const c_char;
    }

    #[cfg(not(target_os = "linux"))]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> i32;
        fn dlerror() -> *const c_char;
    }

    const RTLD_NOW: i32 = 2;

    pub struct OsLibrary {
        handle: *mut c_void,
    }

    unsafe impl Send for OsLibrary {}
    unsafe impl Sync for OsLibrary {}

    impl OsLibrary {
        pub fn load(path: &Path) -> Result<Self, String> {
            let c_path = CString::new(path.to_str().unwrap_or("")).map_err(|e| e.to_string())?;
            let handle = unsafe { dlopen(c_path.as_ptr(), RTLD_NOW) };
            if handle.is_null() {
                let err_detail = unsafe {
                    let err = dlerror();
                    if !err.is_null() {
                        CStr::from_ptr(err).to_string_lossy().into_owned()
                    } else {
                        "unknown dynamic link error".to_string()
                    }
                };
                Err(format!("dlopen failed for {}: {}", path.display(), err_detail))
            } else {
                Ok(Self { handle })
            }
        }

        pub unsafe fn get_symbol<T>(&self, symbol: &[u8]) -> Result<T, String> {
            let proc = unsafe { dlsym(self.handle, symbol.as_ptr() as *const c_char) };
            if proc.is_null() {
                let err_detail = unsafe {
                    let err = dlerror();
                    if !err.is_null() {
                        CStr::from_ptr(err).to_string_lossy().into_owned()
                    } else {
                        "symbol not found".to_string()
                    }
                };
                Err(format!(
                    "Symbol '{}' not found: {}",
                    String::from_utf8_lossy(symbol),
                    err_detail
                ))
            } else {
                Ok(std::mem::transmute_copy(&proc))
            }
        }
    }

    impl Drop for OsLibrary {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                unsafe {
                    dlclose(self.handle);
                }
                self.handle = std::ptr::null_mut();
            }
        }
    }
}

/// Error types encountered during duct initialization or runtime execution.
#[derive(Debug)]
pub enum DuctError {
    WindowRequired,
    RendererDirectoryNotFound(String),
    NoBackendsAvailable(PathBuf),
    BackendNotFound {
        requested: BackendPreference,
        search_path: PathBuf,
    },
    LibraryLoadError {
        path: PathBuf,
        source: String,
    },
    SymbolResolutionError {
        symbol: String,
        source: String,
    },
    NullVTableReturned,
    PipelineOperationFailed(DuctResult),
    InvalidStringData,
}

impl fmt::Display for DuctError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DuctError::WindowRequired => {
                write!(f, "Pipeline initialization failed: a valid native window (e.g. from windowed::Window) is strictly required")
            }
            DuctError::RendererDirectoryNotFound(msg) => {
                write!(f, "Renderer directory could not be located: {}", msg)
            }
            DuctError::NoBackendsAvailable(path) => {
                write!(f, "No pipeline modules found in renderer directory: {}", path.display())
            }
            DuctError::BackendNotFound { requested, search_path } => {
                write!(
                    f,
                    "Requested backend {:?} was not found in: {}",
                    requested,
                    search_path.display()
                )
            }
            DuctError::LibraryLoadError { path, source } => {
                write!(f, "Failed to dynamically load library at {}: {}", path.display(), source)
            }
            DuctError::SymbolResolutionError { symbol, source } => {
                write!(f, "Failed to resolve symbol '{}': {}", symbol, source)
            }
            DuctError::NullVTableReturned => {
                write!(f, "Dynamic pipeline factory returned a null vtable pointer")
            }
            DuctError::PipelineOperationFailed(res) => {
                write!(f, "Pipeline call failed with result code: {}", res)
            }
            DuctError::InvalidStringData => {
                write!(f, "Failed to convert C-string to valid UTF-8")
            }
        }
    }
}

impl std::error::Error for DuctError {}

/// Preferred backend selection.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BackendPreference {
    Auto,
    OpenGL,
    DirectX12,
    Vulkan,
}

impl fmt::Display for BackendPreference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendPreference::Auto => write!(f, "Auto"),
            BackendPreference::OpenGL => write!(f, "OpenGL"),
            BackendPreference::DirectX12 => write!(f, "DirectX 12"),
            BackendPreference::Vulkan => write!(f, "Vulkan"),
        }
    }
}

/// Low-level raw OpenGL context escape hatch.
pub struct RawGlContext {
    pub hglrc: *mut c_void,
    pub hdc: *mut c_void,
    pub extra_info: String,
    pub get_proc_address: Option<unsafe extern "C" fn(*const c_char) -> *mut c_void>,
}

/// Builder for initializing Duct with explicit parameters or environment discovery.
pub struct DuctBuilder {
    preferred_backend: BackendPreference,
    renderer_dir: Option<PathBuf>,
    window: DuctWindowHandle,
    viewport_width: u32,
    viewport_height: u32,
    vsync: bool,
    debug_mode: bool,
    app_name: String,
}

impl Default for DuctBuilder {
    fn default() -> Self {
        Self {
            preferred_backend: BackendPreference::Auto,
            renderer_dir: None,
            window: DuctWindowHandle::null(),
            viewport_width: 1280,
            viewport_height: 720,
            vsync: true,
            debug_mode: true,
            app_name: "Project32 Game".to_string(),
        }
    }
}

impl DuctBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses command-line arguments to configure preferred backend flags.
    pub fn from_args(mut self) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut i = 1;
        while i < args.len() {
            let arg = &args[i];
            match arg.as_str() {
                "--gl" | "--opengl" => {
                    self.preferred_backend = BackendPreference::OpenGL;
                }
                "--dx12" | "--d3d12" => {
                    self.preferred_backend = BackendPreference::DirectX12;
                }
                "--vk" | "--vulkan" => {
                    self.preferred_backend = BackendPreference::Vulkan;
                }
                "--renderer-dir" if i + 1 < args.len() => {
                    i += 1;
                    self.renderer_dir = Some(PathBuf::from(&args[i]));
                }
                _ => {}
            }
            i += 1;
        }
        self
    }

    /// Attaches a `windowed::Window` directly to the graphics pipeline.
    #[cfg(feature = "windowed")]
    pub fn with_windowed(mut self, window: &windowed::Window) -> Self {
        use windowed::rwd::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
        if let Ok(handle) = window.window_handle() {
            match handle.as_raw() {
                RawWindowHandle::Win32(w) => {
                    let hwnd = w.hwnd.get() as *mut c_void;
                    let hinst = w
                        .hinstance
                        .map(|h| h.get() as *mut c_void)
                        .unwrap_or(std::ptr::null_mut());
                    self.window = DuctWindowHandle::from_win32(hwnd, hinst);
                }
                RawWindowHandle::Xlib(x) => {
                    let mut disp_ptr = std::ptr::null_mut();
                    if let Ok(dh) = window.display_handle() {
                        if let RawDisplayHandle::Xlib(d) = dh.as_raw() {
                            disp_ptr = d.display.map(|p| p.as_ptr()).unwrap_or(std::ptr::null_mut());
                        }
                    }
                    self.window = DuctWindowHandle::from_xlib(x.window, disp_ptr);
                }
                RawWindowHandle::AppKit(a) => {
                    self.window = DuctWindowHandle::from_appkit(a.ns_view.as_ptr());
                }
            }
        }
        let (w, h) = window.inner_size();
        self.viewport_width = w;
        self.viewport_height = h;
        self
    }

    pub fn preferred_backend(mut self, pref: BackendPreference) -> Self {
        self.preferred_backend = pref;
        self
    }

    pub fn renderer_dir<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.renderer_dir = Some(path.into());
        self
    }

    pub fn window(mut self, window: DuctWindowHandle) -> Self {
        self.window = window;
        self
    }

    pub fn raw_window_handles(
        mut self,
        native_window: *mut c_void,
        native_display: *mut c_void,
    ) -> Self {
        self.window = DuctWindowHandle::from_raw(native_window, native_display);
        self
    }

    pub fn viewport(mut self, width: u32, height: u32) -> Self {
        self.viewport_width = width;
        self.viewport_height = height;
        self
    }

    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    pub fn debug_mode(mut self, debug: bool) -> Self {
        self.debug_mode = debug;
        self
    }

    pub fn app_name<S: Into<String>>(mut self, name: S) -> Self {
        self.app_name = name.into();
        self
    }

    /// Discovers candidate modules and initializes the active pipeline.
    pub fn build(self) -> Result<Duct, DuctError> {
        // Enforce that an attached window is provided
        if !self.window.is_valid() {
            return Err(DuctError::WindowRequired);
        }

        let renderer_dir = resolve_renderer_dir(self.renderer_dir.as_deref())?;
        println!("[duct] Renderer directory resolved to: {}", renderer_dir.display());

        let candidates = determine_load_candidates(self.preferred_backend);

        let mut last_error: Option<DuctError> = None;

        for (preference, filenames) in &candidates {
            let mut resolved_path = None;
            for filename in *filenames {
                let p = renderer_dir.join(filename);
                if p.is_file() {
                    resolved_path = Some(p);
                    break;
                }
            }

            let dll_path = match resolved_path {
                Some(p) => p,
                None => {
                    if self.preferred_backend != BackendPreference::Auto
                        && *preference == self.preferred_backend
                    {
                        eprintln!(
                            "[duct][WARN] Backend '{}' requested via CLI was not found in '{}'. Falling back to next available backend...",
                            preference,
                            renderer_dir.display()
                        );
                    }
                    continue;
                }
            };

            println!(
                "[duct] Attempting to load pipeline module from: {}",
                dll_path.display()
            );

            match load_pipeline_from_dll(&dll_path, &self, &renderer_dir) {
                Ok(duct) => {
                    println!(
                        "[duct] Pipeline '{}' v{} loaded successfully!",
                        duct.backend_name(),
                        duct.backend_version()
                    );
                    return Ok(duct);
                }
                Err(err) => {
                    eprintln!(
                        "[duct][WARN] Failed to initialize backend '{}' from {}: {}. Attempting fallback...",
                        preference,
                        dll_path.display(),
                        err
                    );
                    last_error = Some(err);
                }
            }
        }

        if let Some(err) = last_error {
            Err(err)
        } else {
            Err(DuctError::NoBackendsAvailable(renderer_dir))
        }
    }
}

/// Typed GPU buffer resource.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GpuBuffer {
    pub handle: DuctBufferHandle,
    pub buffer_type: DuctBufferType,
    pub size_bytes: usize,
}

impl GpuBuffer {
    #[inline]
    pub fn handle(&self) -> DuctBufferHandle {
        self.handle
    }
}

/// Typed GPU shader resource.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GpuShader {
    pub handle: DuctShaderHandle,
    pub stage: DuctShaderStage,
}

impl GpuShader {
    #[inline]
    pub fn handle(&self) -> DuctShaderHandle {
        self.handle
    }
}

/// Typed GPU 2D texture resource.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GpuTexture {
    pub handle: DuctTextureHandle,
    pub width: u32,
    pub height: u32,
    pub format: DuctTextureFormat,
}

impl GpuTexture {
    #[inline]
    pub fn handle(&self) -> DuctTextureHandle {
        self.handle
    }
}

/// Typed GPU offscreen render target (FBO) resource.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GpuRenderTarget {
    pub handle: DuctRenderTargetHandle,
    pub width: u32,
    pub height: u32,
    pub format: DuctTextureFormat,
    pub has_depth: bool,
}

impl GpuRenderTarget {
    #[inline]
    pub fn handle(&self) -> DuctRenderTargetHandle {
        self.handle
    }
}

/// Main controller struct wrapping dynamic pipeline state and routing calls.
pub struct Duct {
    vtable: *mut DuctPipelineVTable,
    _library: os_lib::OsLibrary,
    backend_type: DuctBackendType,
    backend_name: String,
    backend_version: String,
    name_cstr: CString,
    window: DuctWindowHandle,
    viewport_width: u32,
    viewport_height: u32,
    vsync: bool,
    debug_mode: bool,
    app_name: String,
    renderer_dir: PathBuf,
    shape_vbo: DuctBufferHandle,
    shape_vertex_cursor: u32,
    last_bound_user_vbo: DuctBufferHandle,
}

unsafe impl Send for Duct {}

impl Duct {
    /// Convenience initializer inspecting `std::env::args()` with an attached window.
    pub fn init_from_args(window: DuctWindowHandle) -> Result<Self, DuctError> {
        DuctBuilder::new().window(window).from_args().build()
    }

    /// Convenience initializer when using `windowed`.
    #[cfg(feature = "windowed")]
    pub fn init_windowed(window: &windowed::Window) -> Result<Self, DuctError> {
        DuctBuilder::new().with_windowed(window).from_args().build()
    }

    /// Creates a new builder instance.
    pub fn builder() -> DuctBuilder {
        DuctBuilder::new()
    }

    #[inline]
    pub fn backend_type(&self) -> DuctBackendType {
        self.backend_type
    }

    #[inline]
    pub fn backend_name(&self) -> &str {
        &self.backend_name
    }

    #[inline]
    pub fn backend_version(&self) -> &str {
        &self.backend_version
    }

    #[inline]
    fn vtable(&self) -> &DuctPipelineVTable {
        unsafe { &*self.vtable }
    }

    /// Resizes the pipeline viewport.
    pub fn resize_viewport(&mut self, width: u32, height: u32) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.resize_viewport)(v.context, width, height) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Sets the scissor rectangle.
    pub fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.set_scissor)(v.context, x, y, width, height) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Begins a new frame with the specified clear color.
    pub fn begin_frame(&mut self, clear_color: DuctColor) -> Result<(), DuctError> {
        self.shape_vertex_cursor = 0;
        let v = self.vtable();
        let res = unsafe { (v.begin_frame)(v.context, clear_color) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Ends the current frame.
    pub fn end_frame(&mut self) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.end_frame)(v.context) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Presents the rendered frame to the display.
    pub fn present(&mut self) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.present)(v.context) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Creates a GPU shader from source.
    pub fn create_shader(
        &mut self,
        stage: DuctShaderStage,
        source: &str,
        entry_point: Option<&str>,
    ) -> Result<DuctShaderHandle, DuctError> {
        let entry_cstr = entry_point.map(|e| CString::new(e).unwrap_or_default());
        let desc = DuctShaderDesc {
            stage,
            source_bytes: source.as_ptr(),
            source_len: source.len(),
            entry_point: entry_cstr
                .as_ref()
                .map(|c| c.as_ptr())
                .unwrap_or(std::ptr::null()),
        };

        let mut handle = DuctShaderHandle::INVALID;
        let v = self.vtable();
        let res = unsafe { (v.create_shader)(v.context, &desc, &mut handle) };
        if res.is_ok() {
            Ok(handle)
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Destroys a previously created shader.
    pub fn destroy_shader(&mut self, handle: DuctShaderHandle) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.destroy_shader)(v.context, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Creates a GPU buffer.
    pub fn create_buffer(
        &mut self,
        buffer_type: DuctBufferType,
        size_bytes: usize,
        initial_data: Option<&[u8]>,
        is_dynamic: bool,
    ) -> Result<DuctBufferHandle, DuctError> {
        let data_ptr = initial_data
            .map(|d| d.as_ptr())
            .unwrap_or(std::ptr::null());

        let desc = DuctBufferDesc {
            buffer_type,
            size_bytes,
            initial_data: data_ptr,
            is_dynamic,
        };

        let mut handle = DuctBufferHandle::INVALID;
        let v = self.vtable();
        let res = unsafe { (v.create_buffer)(v.context, &desc, &mut handle) };
        if res.is_ok() {
            if buffer_type == DuctBufferType::Vertex {
                self.last_bound_user_vbo = handle;
            }
            Ok(handle)
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Updates existing buffer contents.
    pub fn update_buffer(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.update_buffer)(v.context, handle, offset, data.as_ptr(), data.len()) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Destroys an allocated buffer.
    pub fn destroy_buffer(&mut self, handle: DuctBufferHandle) -> Result<(), DuctError> {
        if self.last_bound_user_vbo == handle {
            self.last_bound_user_vbo = DuctBufferHandle::INVALID;
        }
        if self.shape_vbo == handle {
            self.shape_vbo = DuctBufferHandle::INVALID;
        }
        let v = self.vtable();
        let res = unsafe { (v.destroy_buffer)(v.context, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Creates a 2D GPU texture.
    pub fn create_texture_2d(
        &mut self,
        width: u32,
        height: u32,
        format: DuctTextureFormat,
        data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctError> {
        let ptr = data.map(|d| d.as_ptr()).unwrap_or(std::ptr::null());
        let mut handle = DuctTextureHandle::INVALID;
        let v = self.vtable();
        let res = unsafe { (v.create_texture_2d)(v.context, width, height, format, ptr, &mut handle) };
        if res.is_ok() {
            Ok(handle)
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Destroys a texture.
    pub fn destroy_texture(&mut self, handle: DuctTextureHandle) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.destroy_texture)(v.context, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Binds a texture to a texture unit slot.
    pub fn bind_texture(&mut self, slot: u32, handle: DuctTextureHandle) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.bind_texture)(v.context, slot, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Binds active pipeline state (shaders, blending, culling, depth test).
    pub fn set_pipeline_state(&mut self, desc: &DuctPipelineStateDesc) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.set_pipeline_state)(v.context, desc) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Toggles wireframe polygon rasterization mode.
    pub fn set_wireframe(&mut self, enabled: bool) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.set_wireframe)(v.context, enabled) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Sets a float uniform.
    pub fn set_uniform_float(&mut self, name: &str, val: f32) -> Result<(), DuctError> {
        let c_str = CString::new(name).map_err(|_| DuctError::InvalidStringData)?;
        let v = self.vtable();
        let res = unsafe { (v.set_uniform_float)(v.context, c_str.as_ptr(), val) };
        if res.is_ok() { Ok(()) } else { Err(DuctError::PipelineOperationFailed(res)) }
    }

    /// Sets a 2-component vector uniform.
    pub fn set_uniform_vec2(&mut self, name: &str, x: f32, y: f32) -> Result<(), DuctError> {
        let c_str = CString::new(name).map_err(|_| DuctError::InvalidStringData)?;
        let v = self.vtable();
        let res = unsafe { (v.set_uniform_vec2)(v.context, c_str.as_ptr(), x, y) };
        if res.is_ok() { Ok(()) } else { Err(DuctError::PipelineOperationFailed(res)) }
    }

    /// Sets a 3-component vector uniform.
    pub fn set_uniform_vec3(&mut self, name: &str, x: f32, y: f32, z: f32) -> Result<(), DuctError> {
        let c_str = CString::new(name).map_err(|_| DuctError::InvalidStringData)?;
        let v = self.vtable();
        let res = unsafe { (v.set_uniform_vec3)(v.context, c_str.as_ptr(), x, y, z) };
        if res.is_ok() { Ok(()) } else { Err(DuctError::PipelineOperationFailed(res)) }
    }

    /// Sets a 4-component vector uniform.
    pub fn set_uniform_vec4(&mut self, name: &str, x: f32, y: f32, z: f32, w: f32) -> Result<(), DuctError> {
        let c_str = CString::new(name).map_err(|_| DuctError::InvalidStringData)?;
        let v = self.vtable();
        let res = unsafe { (v.set_uniform_vec4)(v.context, c_str.as_ptr(), x, y, z, w) };
        if res.is_ok() { Ok(()) } else { Err(DuctError::PipelineOperationFailed(res)) }
    }

    /// Sets a 4x4 matrix uniform.
    pub fn set_uniform_mat4(&mut self, name: &str, elements: &[f32; 16]) -> Result<(), DuctError> {
        let c_str = CString::new(name).map_err(|_| DuctError::InvalidStringData)?;
        let v = self.vtable();
        let res = unsafe { (v.set_uniform_mat4)(v.context, c_str.as_ptr(), elements.as_ptr()) };
        if res.is_ok() { Ok(()) } else { Err(DuctError::PipelineOperationFailed(res)) }
    }

    /// Non-indexed draw call.
    pub fn draw(&mut self, vertex_count: u32, first_vertex: u32) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.draw)(v.context, vertex_count, first_vertex) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Indexed draw call.
    pub fn draw_indexed(&mut self, index_count: u32, first_index: u32) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.draw_indexed)(v.context, index_count, first_index) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Reloads a shader dynamically without restarting the engine.
    pub fn reload_shader(
        &mut self,
        handle: DuctShaderHandle,
        stage: DuctShaderStage,
        source: &str,
        entry_point: Option<&str>,
    ) -> Result<(), DuctError> {
        let entry_cstr = entry_point.map(|e| CString::new(e).unwrap_or_default());
        let desc = DuctShaderDesc {
            stage,
            source_bytes: source.as_ptr(),
            source_len: source.len(),
            entry_point: entry_cstr
                .as_ref()
                .map(|c| c.as_ptr())
                .unwrap_or(std::ptr::null()),
        };
        let v = self.vtable();
        let res = unsafe { (v.reload_shader)(v.context, handle, &desc) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Generates complete mipmap chain for a GPU texture.
    pub fn generate_mipmaps(&mut self, handle: DuctTextureHandle) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.generate_mipmaps)(v.context, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Configures texture minification and magnification filtering modes.
    pub fn set_texture_filter(
        &mut self,
        handle: DuctTextureHandle,
        min: DuctFilterMode,
        mag: DuctFilterMode,
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.set_texture_filter)(v.context, handle, min, mag) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Allocates an offscreen render target (FBO).
    pub fn create_render_target(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctError> {
        let mut handle = DuctRenderTargetHandle::INVALID;
        let v = self.vtable();
        let res = unsafe { (v.create_render_target)(v.context, desc, &mut handle) };
        if res.is_ok() {
            Ok(handle)
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Destroys an offscreen render target.
    pub fn destroy_render_target(&mut self, handle: DuctRenderTargetHandle) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.destroy_render_target)(v.context, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Binds an offscreen render target (or DuctRenderTargetHandle::INVALID for window backbuffer).
    pub fn bind_render_target(&mut self, handle: DuctRenderTargetHandle) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.bind_render_target)(v.context, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Retrieves readable color texture handle from an offscreen render target.
    pub fn get_render_target_texture(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctError> {
        let mut tex = DuctTextureHandle::INVALID;
        let v = self.vtable();
        let res = unsafe { (v.get_render_target_texture)(v.context, handle, &mut tex) };
        if res.is_ok() {
            Ok(tex)
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Instanced draw call.
    pub fn draw_instanced(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe {
            (v.draw_instanced)(v.context, vertex_count, instance_count, first_vertex, first_instance)
        };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Indexed instanced draw call with configurable index format (U16 or U32).
    pub fn draw_indexed_instanced(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        index_format: DuctIndexFormat,
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe {
            (v.draw_indexed_instanced)(
                v.context,
                index_count,
                instance_count,
                first_index,
                index_format,
            )
        };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Queries real-time frame telemetry, GPU metrics, and rendering stats.
    pub fn frame_stats(&mut self) -> Result<DuctFrameStats, DuctError> {
        let mut stats = DuctFrameStats::default();
        let v = self.vtable();
        let res = unsafe { (v.get_frame_stats)(v.context, &mut stats) };
        if res.is_ok() {
            Ok(stats)
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Binds a vertex buffer as the active geometry stream.
    pub fn bind_vertex_buffer(&mut self, handle: DuctBufferHandle) -> Result<(), DuctError> {
        if handle != self.shape_vbo {
            self.last_bound_user_vbo = handle;
        }
        let v = self.vtable();
        let res = unsafe { (v.bind_vertex_buffer)(v.context, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Binds an index buffer with the specified element format (`U16` or `U32`).
    pub fn bind_index_buffer(
        &mut self,
        handle: DuctBufferHandle,
        format: DuctIndexFormat,
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.bind_index_buffer)(v.context, handle, format) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Binds a uniform buffer object (UBO) to a shader binding slot.
    pub fn bind_uniform_buffer(&mut self, slot: u32, handle: DuctBufferHandle) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.bind_uniform_buffer)(v.context, slot, handle) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Updates a 2D sub-region of an existing texture.
    pub fn update_texture_2d(
        &mut self,
        handle: DuctTextureHandle,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe {
            (v.update_texture_2d)(v.context, handle, x, y, width, height, data.as_ptr(), data.len())
        };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Configures texture S and T coordinate wrapping modes.
    pub fn set_texture_wrap(
        &mut self,
        handle: DuctTextureHandle,
        wrap_s: DuctWrapMode,
        wrap_t: DuctWrapMode,
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.set_texture_wrap)(v.context, handle, wrap_s, wrap_t) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Blits (copies/resolves) color pixels from `src` render target to `dst` render target.
    pub fn blit_render_target(
        &mut self,
        src: DuctRenderTargetHandle,
        dst: DuctRenderTargetHandle,
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        filter: DuctFilterMode,
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe {
            (v.blit_render_target)(v.context, src, dst, src_w, src_h, dst_w, dst_h, filter)
        };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Reads RGBA8 pixels from the currently bound framebuffer/render target into `out_rgba`.
    pub fn read_pixels(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        out_rgba: &mut [u8],
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe {
            (v.read_pixels)(
                v.context,
                x,
                y,
                width,
                height,
                out_rgba.as_mut_ptr(),
                out_rgba.len(),
            )
        };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Toggles vertical sync (VSync) swap interval at runtime.
    pub fn set_vsync(&mut self, enabled: bool) -> Result<(), DuctError> {
        self.vsync = enabled;
        let v = self.vtable();
        let res = unsafe { (v.set_vsync)(v.context, enabled) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Sets an integer or sampler slot uniform.
    pub fn set_uniform_int(&mut self, name: &str, val: i32) -> Result<(), DuctError> {
        let c_str = CString::new(name).map_err(|_| DuctError::InvalidStringData)?;
        let v = self.vtable();
        let res = unsafe { (v.set_uniform_int)(v.context, c_str.as_ptr(), val) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Dispatches a compute shader workload across `(groups_x, groups_y, groups_z)` workgroups.
    pub fn dispatch_compute(
        &mut self,
        groups_x: u32,
        groups_y: u32,
        groups_z: u32,
    ) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.dispatch_compute)(v.context, groups_x, groups_y, groups_z) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Live switches the rendering backend at runtime without recreating the window.
    pub fn switch_backend(&mut self, pref: BackendPreference) -> Result<(), DuctError> {
        println!("[duct] Switching active rendering backend to {:?}...", pref);

        let candidates = determine_load_candidates(pref);
        let mut resolved = None;
        for (cand_pref, filenames) in &candidates {
            if *cand_pref != pref {
                continue;
            }
            for filename in *filenames {
                let p = self.renderer_dir.join(filename);
                if p.is_file() {
                    resolved = Some(p);
                    break;
                }
            }
            if resolved.is_some() {
                break;
            }
        }

        let dll_path = match resolved {
            Some(p) => p,
            None => {
                return Err(DuctError::BackendNotFound {
                    requested: pref,
                    search_path: self.renderer_dir.clone(),
                });
            }
        };

        // Gracefully shutdown current backend
        self.shape_vbo = DuctBufferHandle::INVALID;
        self.shape_vertex_cursor = 0;
        self.last_bound_user_vbo = DuctBufferHandle::INVALID;
        if !self.vtable.is_null() {
            unsafe {
                let v = &*self.vtable;
                (v.shutdown)(v.context);
                (v.destroy)(self.vtable);
            }
            self.vtable = std::ptr::null_mut();
        }

        // Load new dynamic library
        let lib = os_lib::OsLibrary::load(&dll_path).map_err(|e| DuctError::LibraryLoadError {
            path: dll_path.clone(),
            source: e,
        })?;

        let create_pipeline: DuctCreatePipelineFn = unsafe {
            lib.get_symbol(b"duct_create_pipeline\0")
                .map_err(|e| DuctError::SymbolResolutionError {
                    symbol: "duct_create_pipeline".to_string(),
                    source: e,
                })?
        };

        let vtable = unsafe { create_pipeline() };
        if vtable.is_null() {
            return Err(DuctError::NullVTableReturned);
        }

        let v = unsafe { &*vtable };
        let app_name_cstr = CString::new(self.app_name.as_str()).unwrap_or_default();
        let init_config = DuctInitConfig {
            window: self.window,
            width: self.viewport_width,
            height: self.viewport_height,
            vsync: self.vsync,
            debug_mode: self.debug_mode,
            app_name: app_name_cstr.as_ptr(),
        };

        let init_res = unsafe { (v.init)(v.context, &init_config) };
        if init_res.is_err() {
            unsafe { (v.destroy)(vtable); }
            return Err(DuctError::PipelineOperationFailed(init_res));
        }

        let backend_name = if !v.name.is_null() {
            unsafe { CStr::from_ptr(v.name) }.to_string_lossy().into_owned()
        } else {
            "Unknown".to_string()
        };

        let backend_version = if !v.version.is_null() {
            unsafe { CStr::from_ptr(v.version) }.to_string_lossy().into_owned()
        } else {
            "0.0.0".to_string()
        };

        self.name_cstr = CString::new(backend_name.as_str()).unwrap_or_default();
        self.backend_name = backend_name;
        self.backend_version = backend_version;
        self.backend_type = v.backend_type;
        self.vtable = vtable;
        self._library = lib;

        println!(
            "[duct] Successfully switched to backend: '{}' v{}!",
            self.backend_name, self.backend_version
        );

        Ok(())
    }

    /// General escape hatch querying raw backend device and context pointers.
    pub fn raw_context(&mut self) -> Result<DuctRawContext, DuctError> {
        let v = self.vtable();
        let mut raw = DuctRawContext {
            backend_type: self.backend_type,
            context_handle: std::ptr::null_mut(),
            device_handle: std::ptr::null_mut(),
            extra_info: std::ptr::null(),
            get_proc_address: None,
        };

        let res = unsafe { (v.get_raw_context)(v.context, &mut raw) };
        if res.is_ok() {
            Ok(raw)
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    /// Specialized escape hatch for OpenGL backend, giving direct access to HGLRC, HDC, and proc queries.
    pub fn raw_gl_context(&mut self) -> Option<RawGlContext> {
        if self.backend_type != DuctBackendType::OpenGL {
            return None;
        }

        let raw = self.raw_context().ok()?;
        let extra_info = if !raw.extra_info.is_null() {
            unsafe { CStr::from_ptr(raw.extra_info) }
                .to_string_lossy()
                .into_owned()
        } else {
            String::new()
        };

        Some(RawGlContext {
            hglrc: raw.context_handle,
            hdc: raw.device_handle,
            extra_info,
            get_proc_address: raw.get_proc_address,
        })
    }

    /// Executes a custom driver command on the backend.
    pub fn execute_custom(&mut self, command_id: u32, data: *mut c_void) -> Result<(), DuctError> {
        let v = self.vtable();
        let res = unsafe { (v.execute_custom)(v.context, command_id, data) };
        if res.is_ok() {
            Ok(())
        } else {
            Err(DuctError::PipelineOperationFailed(res))
        }
    }

    // ── Base Shape Geometry & Immediate Drawing Helpers ──────────────────────

    const SHAPE_VBO_MAX_VERTICES: u32 = 4096;

    fn draw_shape_vertices(
        &mut self,
        vertices: &[f32],
        color: DuctColor,
    ) -> Result<(), DuctError> {
        let vertex_count = (vertices.len() / 3) as u32;
        if vertex_count == 0 {
            return Ok(());
        }

        let prev_user_vbo = self.last_bound_user_vbo;

        if !self.shape_vbo.is_valid() {
            let cap_bytes = (Self::SHAPE_VBO_MAX_VERTICES as usize) * 12;
            let desc = DuctBufferDesc {
                buffer_type: DuctBufferType::Vertex,
                size_bytes: cap_bytes,
                initial_data: std::ptr::null(),
                is_dynamic: true,
            };
            let mut handle = DuctBufferHandle::INVALID;
            let v = self.vtable();
            let res = unsafe { (v.create_buffer)(v.context, &desc, &mut handle) };
            if res.is_err() {
                return Err(DuctError::PipelineOperationFailed(res));
            }
            self.shape_vbo = handle;
        }

        if self.shape_vertex_cursor + vertex_count > Self::SHAPE_VBO_MAX_VERTICES {
            self.shape_vertex_cursor = 0;
        }

        let first_vertex = self.shape_vertex_cursor;
        let byte_offset = (first_vertex as usize) * 12;
        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                std::mem::size_of_val(vertices),
            )
        };

        self.update_buffer(self.shape_vbo, byte_offset, byte_slice)?;
        let v = self.vtable();
        let _ = unsafe { (v.bind_vertex_buffer)(v.context, self.shape_vbo) };
        self.set_uniform_vec4("uColor", color.r, color.g, color.b, color.a)?;
        self.draw(vertex_count, first_vertex)?;
        self.shape_vertex_cursor += vertex_count;

        if prev_user_vbo.is_valid() {
            let v = self.vtable();
            let _ = unsafe { (v.bind_vertex_buffer)(v.context, prev_user_vbo) };
            self.last_bound_user_vbo = prev_user_vbo;
        }

        Ok(())
    }

    /// Immediately draws a 2D/3D triangle in normalized device coordinates `[-1.0, 1.0]`.
    pub fn draw_triangle(
        &mut self,
        p0: [f32; 3],
        p1: [f32; 3],
        p2: [f32; 3],
        color: DuctColor,
    ) -> Result<(), DuctError> {
        let verts = [
            p0[0], p0[1], p0[2],
            p1[0], p1[1], p1[2],
            p2[0], p2[1], p2[2],
        ];
        self.draw_shape_vertices(&verts, color)
    }

    /// Immediately draws a filled axis-aligned rectangle (2 triangles, 6 vertices) in NDC `[-1.0, 1.0]`.
    /// `(x, y)` is the bottom-left corner and `(width, height)` are the dimensions in NDC.
    pub fn draw_rect(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: DuctColor,
    ) -> Result<(), DuctError> {
        let x0 = x;
        let y0 = y;
        let x1 = x + width;
        let y1 = y + height;
        let verts = [
            // Triangle 1
            x0, y0, 0.0,
            x1, y0, 0.0,
            x1, y1, 0.0,
            // Triangle 2
            x0, y0, 0.0,
            x1, y1, 0.0,
            x0, y1, 0.0,
        ];
        self.draw_shape_vertices(&verts, color)
    }

    /// Immediately draws a filled circle or regular polygon (triangle fan decomposed into `segments` triangles)
    /// in NDC `[-1.0, 1.0]`.
    pub fn draw_circle(
        &mut self,
        center_x: f32,
        center_y: f32,
        radius: f32,
        segments: u32,
        color: DuctColor,
    ) -> Result<(), DuctError> {
        let segs = segments.clamp(3, 256);
        let mut verts = Vec::with_capacity((segs as usize) * 9);
        let step = std::f32::consts::TAU / (segs as f32);
        for i in 0..segs {
            let a0 = (i as f32) * step;
            let a1 = ((i + 1) as f32) * step;
            verts.extend_from_slice(&[
                center_x,
                center_y,
                0.0,
                center_x + radius * a0.cos(),
                center_y + radius * a0.sin(),
                0.0,
                center_x + radius * a1.cos(),
                center_y + radius * a1.sin(),
                0.0,
            ]);
        }
        self.draw_shape_vertices(&verts, color)
    }

    /// Allocates a static GPU vertex buffer containing a single triangle (`3` vertices, `Float3`).
    pub fn create_triangle_buffer(
        &mut self,
        p0: [f32; 3],
        p1: [f32; 3],
        p2: [f32; 3],
    ) -> Result<DuctBufferHandle, DuctError> {
        let verts: [f32; 9] = [
            p0[0], p0[1], p0[2],
            p1[0], p1[1], p1[2],
            p2[0], p2[1], p2[2],
        ];
        let bytes = unsafe {
            std::slice::from_raw_parts(verts.as_ptr() as *const u8, std::mem::size_of_val(&verts))
        };
        self.create_buffer(DuctBufferType::Vertex, bytes.len(), Some(bytes), false)
    }

    /// Allocates static GPU vertex and index (`U16`) buffers for a quad/rectangle (`4` vertices, `6` indices).
    pub fn create_rect_buffers(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<(DuctBufferHandle, DuctBufferHandle), DuctError> {
        let verts: [f32; 12] = [
            x,         y,          0.0,
            x + width, y,          0.0,
            x + width, y + height, 0.0,
            x,         y + height, 0.0,
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let v_bytes = unsafe {
            std::slice::from_raw_parts(verts.as_ptr() as *const u8, std::mem::size_of_val(&verts))
        };
        let i_bytes = unsafe {
            std::slice::from_raw_parts(
                indices.as_ptr() as *const u8,
                std::mem::size_of_val(&indices),
            )
        };
        let vbo = self.create_buffer(DuctBufferType::Vertex, v_bytes.len(), Some(v_bytes), false)?;
        let ibo = self.create_buffer(DuctBufferType::Index, i_bytes.len(), Some(i_bytes), false)?;
        Ok((vbo, ibo))
    }

    /// Allocates a static GPU vertex buffer for a filled circle (`segments * 3` vertices) and returns `(vbo, vertex_count)`.
    pub fn create_circle_buffer(
        &mut self,
        center_x: f32,
        center_y: f32,
        radius: f32,
        segments: u32,
    ) -> Result<(DuctBufferHandle, u32), DuctError> {
        let segs = segments.clamp(3, 256);
        let mut verts = Vec::with_capacity((segs as usize) * 9);
        let step = std::f32::consts::TAU / (segs as f32);
        for i in 0..segs {
            let a0 = (i as f32) * step;
            let a1 = ((i + 1) as f32) * step;
            verts.extend_from_slice(&[
                center_x,
                center_y,
                0.0,
                center_x + radius * a0.cos(),
                center_y + radius * a0.sin(),
                0.0,
                center_x + radius * a1.cos(),
                center_y + radius * a1.sin(),
                0.0,
            ]);
        }
        let bytes = unsafe {
            std::slice::from_raw_parts(
                verts.as_ptr() as *const u8,
                verts.len() * std::mem::size_of::<f32>(),
            )
        };
        let vbo = self.create_buffer(DuctBufferType::Vertex, bytes.len(), Some(bytes), false)?;
        Ok((vbo, segs * 3))
    }
}

impl Drop for Duct {
    fn drop(&mut self) {
        if !self.vtable.is_null() {
            unsafe {
                let v = &*self.vtable;
                println!("[duct] Gracefully shutting down pipeline '{}'...", self.backend_name);
                (v.shutdown)(v.context);
                (v.destroy)(self.vtable);
            }
            self.vtable = std::ptr::null_mut();
        }
    }
}

/// Helper locating the renderer directory across common locations.
fn resolve_renderer_dir(explicit: Option<&Path>) -> Result<PathBuf, DuctError> {
    if let Some(path) = explicit {
        if path.is_dir() {
            return Ok(path.to_path_buf());
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let next_to_exe = exe_dir.join("renderer");
            if next_to_exe.is_dir() {
                return Ok(next_to_exe);
            }

            if let Some(target_dir) = exe_dir.parent() {
                let next_to_target = target_dir.join("renderer");
                if next_to_target.is_dir() {
                    return Ok(next_to_target);
                }
                if let Some(workspace_dir) = target_dir.parent() {
                    let workspace_renderer = workspace_dir.join("renderer");
                    if workspace_renderer.is_dir() {
                        return Ok(workspace_renderer);
                    }
                }
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let cwd_renderer = cwd.join("renderer");
        if cwd_renderer.is_dir() {
            return Ok(cwd_renderer);
        }
        let target_debug_renderer = cwd.join("target").join("debug").join("renderer");
        if target_debug_renderer.is_dir() {
            return Ok(target_debug_renderer);
        }
        let target_release_renderer = cwd.join("target").join("release").join("renderer");
        if target_release_renderer.is_dir() {
            return Ok(target_release_renderer);
        }
    }

    let fallback = PathBuf::from("./renderer");
    if fallback.is_dir() {
        Ok(fallback)
    } else {
        Err(DuctError::RendererDirectoryNotFound(
            "Could not locate './renderer/' in executable path or current directory".to_string(),
        ))
    }
}

fn determine_load_candidates(
    pref: BackendPreference,
) -> Vec<(BackendPreference, &'static [&'static str])> {
    const GL_NAMES: &[&str] = &[
        "duct.gl.dll",
        "glduct.dll",
        "libduct.gl.so",
        "duct.gl.so",
        "libglduct.so",
        "duct.gl.dylib",
        "libduct.gl.dylib",
        "libglduct.dylib",
    ];
    const DX12_NAMES: &[&str] = &[
        "duct.dx12.dll",
        "dx12duct.dll",
        "libduct.dx12.so",
        "duct.dx12.so",
        "libdx12duct.so",
        "duct.dx12.dylib",
        "libduct.dx12.dylib",
        "libdx12duct.dylib",
    ];
    const VK_NAMES: &[&str] = &[
        "duct.vk.dll",
        "vkduct.dll",
        "libduct.vk.so",
        "duct.vk.so",
        "libvkduct.so",
        "duct.vk.dylib",
        "libduct.vk.dylib",
        "libvkduct.dylib",
    ];

    match pref {
        BackendPreference::DirectX12 => vec![
            (BackendPreference::DirectX12, DX12_NAMES),
            (BackendPreference::OpenGL, GL_NAMES),
            (BackendPreference::Vulkan, VK_NAMES),
        ],
        BackendPreference::Vulkan => vec![
            (BackendPreference::Vulkan, VK_NAMES),
            (BackendPreference::OpenGL, GL_NAMES),
            (BackendPreference::DirectX12, DX12_NAMES),
        ],
        BackendPreference::OpenGL => vec![
            (BackendPreference::OpenGL, GL_NAMES),
            (BackendPreference::Vulkan, VK_NAMES),
            (BackendPreference::DirectX12, DX12_NAMES),
        ],
        BackendPreference::Auto => {
            #[cfg(target_os = "windows")]
            {
                vec![
                    (BackendPreference::OpenGL, GL_NAMES),
                    (BackendPreference::Vulkan, VK_NAMES),
                    (BackendPreference::DirectX12, DX12_NAMES),
                ]
            }
            #[cfg(not(target_os = "windows"))]
            {
                vec![
                    (BackendPreference::OpenGL, GL_NAMES),
                    (BackendPreference::Vulkan, VK_NAMES),
                ]
            }
        }
    }
}

fn load_pipeline_from_dll(path: &Path, builder: &DuctBuilder, renderer_dir: &Path) -> Result<Duct, DuctError> {
    let lib = os_lib::OsLibrary::load(path).map_err(|e| DuctError::LibraryLoadError {
        path: path.to_path_buf(),
        source: e,
    })?;

    let create_pipeline: DuctCreatePipelineFn = unsafe {
        lib.get_symbol(b"duct_create_pipeline\0")
            .map_err(|e| DuctError::SymbolResolutionError {
                symbol: "duct_create_pipeline".to_string(),
                source: e,
            })?
    };

    let vtable = unsafe { create_pipeline() };
    if vtable.is_null() {
        return Err(DuctError::NullVTableReturned);
    }

    let v = unsafe { &*vtable };

    let app_name_cstr = CString::new(builder.app_name.as_str()).unwrap_or_default();
    let init_config = DuctInitConfig {
        window: builder.window,
        width: builder.viewport_width,
        height: builder.viewport_height,
        vsync: builder.vsync,
        debug_mode: builder.debug_mode,
        app_name: app_name_cstr.as_ptr(),
    };

    let init_res = unsafe { (v.init)(v.context, &init_config) };
    if init_res.is_err() {
        unsafe {
            (v.destroy)(vtable);
        }
        return Err(DuctError::PipelineOperationFailed(init_res));
    }

    let backend_name = if !v.name.is_null() {
        unsafe { CStr::from_ptr(v.name) }
            .to_string_lossy()
            .into_owned()
    } else {
        "Unknown".to_string()
    };

    let backend_version = if !v.version.is_null() {
        unsafe { CStr::from_ptr(v.version) }
            .to_string_lossy()
            .into_owned()
    } else {
        "0.0.0".to_string()
    };

    let name_cstr = CString::new(backend_name.as_str()).unwrap_or_default();

    Ok(Duct {
        vtable,
        _library: lib,
        backend_type: v.backend_type,
        backend_name,
        backend_version,
        name_cstr,
        window: builder.window,
        viewport_width: builder.viewport_width,
        viewport_height: builder.viewport_height,
        vsync: builder.vsync,
        debug_mode: builder.debug_mode,
        app_name: builder.app_name.clone(),
        renderer_dir: renderer_dir.to_path_buf(),
        shape_vbo: DuctBufferHandle::INVALID,
        shape_vertex_cursor: 0,
        last_bound_user_vbo: DuctBufferHandle::INVALID,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// C-ABI Exported API for External Language Support (C, C++, C#, Python, etc.)
// ─────────────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn duct_create(
    config: *const DuctInitConfig,
    out_handle: *mut *mut Duct,
) -> DuctResult {
    if config.is_null() || out_handle.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let cfg = unsafe { &*config };
    let builder = DuctBuilder::new()
        .window(cfg.window)
        .viewport(cfg.width, cfg.height)
        .vsync(cfg.vsync)
        .debug_mode(cfg.debug_mode);

    match builder.build() {
        Ok(duct) => {
            unsafe { *out_handle = Box::into_raw(Box::new(duct)) };
            DuctResult::Success
        }
        Err(DuctError::WindowRequired) => DuctResult::ErrorWindowRequired,
        Err(_) => DuctResult::ErrorInitializationFailed,
    }
}

#[no_mangle]
pub extern "C" fn duct_create_from_args(
    window: DuctWindowHandle,
    out_handle: *mut *mut Duct,
) -> DuctResult {
    if out_handle.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    match Duct::init_from_args(window) {
        Ok(duct) => {
            unsafe { *out_handle = Box::into_raw(Box::new(duct)) };
            DuctResult::Success
        }
        Err(DuctError::WindowRequired) => DuctResult::ErrorWindowRequired,
        Err(_) => DuctResult::ErrorInitializationFailed,
    }
}

#[no_mangle]
pub extern "C" fn duct_destroy(handle: *mut Duct) {
    if !handle.is_null() {
        let _ = unsafe { Box::from_raw(handle) };
    }
}

#[no_mangle]
pub extern "C" fn duct_resize_viewport(handle: *mut Duct, width: u32, height: u32) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.resize_viewport(width, height) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_scissor(handle: *mut Duct, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.set_scissor(x, y, width, height) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_begin_frame(handle: *mut Duct, clear_color: DuctColor) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.begin_frame(clear_color) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_end_frame(handle: *mut Duct) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.end_frame() {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_present(handle: *mut Duct) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.present() {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_create_shader(
    handle: *mut Duct,
    desc: *const DuctShaderDesc,
    out_shader: *mut DuctShaderHandle,
) -> DuctResult {
    if handle.is_null() || desc.is_null() || out_shader.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let duct = unsafe { &mut *handle };
    let v = duct.vtable();
    unsafe { (v.create_shader)(v.context, desc, out_shader) }
}

#[no_mangle]
pub extern "C" fn duct_destroy_shader(handle: *mut Duct, shader: DuctShaderHandle) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.destroy_shader(shader) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_create_buffer(
    handle: *mut Duct,
    desc: *const DuctBufferDesc,
    out_buffer: *mut DuctBufferHandle,
) -> DuctResult {
    if handle.is_null() || desc.is_null() || out_buffer.is_null() {
        return DuctResult::ErrorInvalidParameter;
    }
    let duct = unsafe { &mut *handle };
    let v = duct.vtable();
    unsafe { (v.create_buffer)(v.context, desc, out_buffer) }
}

#[no_mangle]
pub extern "C" fn duct_update_buffer(
    handle: *mut Duct,
    buffer: DuctBufferHandle,
    offset: usize,
    data: *const u8,
    size: usize,
) -> DuctResult {
    if handle.is_null() || data.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let slice = unsafe { std::slice::from_raw_parts(data, size) };
    match duct.update_buffer(buffer, offset, slice) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_destroy_buffer(handle: *mut Duct, buffer: DuctBufferHandle) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.destroy_buffer(buffer) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_create_texture_2d(
    handle: *mut Duct,
    width: u32,
    height: u32,
    format: DuctTextureFormat,
    data: *const u8,
    out_texture: *mut DuctTextureHandle,
) -> DuctResult {
    if handle.is_null() || out_texture.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let v = duct.vtable();
    unsafe { (v.create_texture_2d)(v.context, width, height, format, data, out_texture) }
}

#[no_mangle]
pub extern "C" fn duct_destroy_texture(handle: *mut Duct, texture: DuctTextureHandle) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.destroy_texture(texture) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_bind_texture(handle: *mut Duct, slot: u32, texture: DuctTextureHandle) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.bind_texture(slot, texture) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_pipeline_state(handle: *mut Duct, desc: *const DuctPipelineStateDesc) -> DuctResult {
    if handle.is_null() || desc.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let d = unsafe { &*desc };
    match duct.set_pipeline_state(d) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_wireframe(handle: *mut Duct, enabled: bool) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.set_wireframe(enabled) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_uniform_float(handle: *mut Duct, name: *const c_char, val: f32) -> DuctResult {
    if handle.is_null() || name.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    match duct.set_uniform_float(name_str, val) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_uniform_vec2(handle: *mut Duct, name: *const c_char, x: f32, y: f32) -> DuctResult {
    if handle.is_null() || name.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    match duct.set_uniform_vec2(name_str, x, y) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_uniform_vec3(handle: *mut Duct, name: *const c_char, x: f32, y: f32, z: f32) -> DuctResult {
    if handle.is_null() || name.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    match duct.set_uniform_vec3(name_str, x, y, z) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_uniform_vec4(handle: *mut Duct, name: *const c_char, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
    if handle.is_null() || name.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    match duct.set_uniform_vec4(name_str, x, y, z, w) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_uniform_mat4(handle: *mut Duct, name: *const c_char, elements_16: *const f32) -> DuctResult {
    if handle.is_null() || name.is_null() || elements_16.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    let slice = unsafe { std::slice::from_raw_parts(elements_16, 16) };
    let mut arr = [0.0f32; 16];
    arr.copy_from_slice(slice);
    match duct.set_uniform_mat4(name_str, &arr) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_draw(handle: *mut Duct, vertex_count: u32, first_vertex: u32) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.draw(vertex_count, first_vertex) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_draw_indexed(handle: *mut Duct, index_count: u32, first_index: u32) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.draw_indexed(index_count, first_index) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_reload_shader(
    handle: *mut Duct,
    shader: DuctShaderHandle,
    desc: *const DuctShaderDesc,
) -> DuctResult {
    if handle.is_null() || desc.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let d = unsafe { &*desc };
    let v = duct.vtable();
    unsafe { (v.reload_shader)(v.context, shader, d) }
}

#[no_mangle]
pub extern "C" fn duct_generate_mipmaps(
    handle: *mut Duct,
    texture: DuctTextureHandle,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.generate_mipmaps(texture) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_texture_filter(
    handle: *mut Duct,
    texture: DuctTextureHandle,
    min_filter: DuctFilterMode,
    mag_filter: DuctFilterMode,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.set_texture_filter(texture, min_filter, mag_filter) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_create_render_target(
    handle: *mut Duct,
    desc: *const DuctRenderTargetDesc,
    out_target: *mut DuctRenderTargetHandle,
) -> DuctResult {
    if handle.is_null() || desc.is_null() || out_target.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let d = unsafe { &*desc };
    match duct.create_render_target(d) {
        Ok(rt) => {
            unsafe { *out_target = rt };
            DuctResult::Success
        }
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_destroy_render_target(
    handle: *mut Duct,
    target: DuctRenderTargetHandle,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.destroy_render_target(target) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_bind_render_target(
    handle: *mut Duct,
    target: DuctRenderTargetHandle,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.bind_render_target(target) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_get_render_target_texture(
    handle: *mut Duct,
    target: DuctRenderTargetHandle,
    out_texture: *mut DuctTextureHandle,
) -> DuctResult {
    if handle.is_null() || out_texture.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.get_render_target_texture(target) {
        Ok(t) => {
            unsafe { *out_texture = t };
            DuctResult::Success
        }
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_draw_instanced(
    handle: *mut Duct,
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.draw_instanced(vertex_count, instance_count, first_vertex, first_instance) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_draw_indexed_instanced(
    handle: *mut Duct,
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    index_format: DuctIndexFormat,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.draw_indexed_instanced(index_count, instance_count, first_index, index_format) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_get_frame_stats(
    handle: *mut Duct,
    out_stats: *mut DuctFrameStats,
) -> DuctResult {
    if handle.is_null() || out_stats.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.frame_stats() {
        Ok(stats) => {
            unsafe { *out_stats = stats };
            DuctResult::Success
        }
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_switch_backend(
    handle: *mut Duct,
    preference: u32,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let pref = match preference {
        1 => BackendPreference::OpenGL,
        2 => BackendPreference::DirectX12,
        3 => BackendPreference::Vulkan,
        _ => BackendPreference::Auto,
    };
    let duct = unsafe { &mut *handle };
    match duct.switch_backend(pref) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_get_backend_name(handle: *mut Duct) -> *const c_char {
    if handle.is_null() { return std::ptr::null(); }
    let duct = unsafe { &*handle };
    duct.name_cstr.as_ptr()
}

#[no_mangle]
pub extern "C" fn duct_get_backend_type(handle: *mut Duct) -> u32 {
    if handle.is_null() { return DuctBackendType::Unknown as u32; }
    let duct = unsafe { &*handle };
    duct.backend_type as u32
}

#[no_mangle]
pub extern "C" fn duct_get_raw_context(handle: *mut Duct, out_raw: *mut DuctRawContext) -> DuctResult {
    if handle.is_null() || out_raw.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.raw_context() {
        Ok(raw) => {
            unsafe { *out_raw = raw };
            DuctResult::Success
        }
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_bind_vertex_buffer(
    handle: *mut Duct,
    buffer: DuctBufferHandle,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.bind_vertex_buffer(buffer) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_bind_index_buffer(
    handle: *mut Duct,
    buffer: DuctBufferHandle,
    format: DuctIndexFormat,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.bind_index_buffer(buffer, format) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_bind_uniform_buffer(
    handle: *mut Duct,
    slot: u32,
    buffer: DuctBufferHandle,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.bind_uniform_buffer(slot, buffer) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_update_texture_2d(
    handle: *mut Duct,
    texture: DuctTextureHandle,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    data: *const u8,
    size: usize,
) -> DuctResult {
    if handle.is_null() || data.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let slice = unsafe { std::slice::from_raw_parts(data, size) };
    match duct.update_texture_2d(texture, x, y, width, height, slice) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_texture_wrap(
    handle: *mut Duct,
    texture: DuctTextureHandle,
    wrap_s: DuctWrapMode,
    wrap_t: DuctWrapMode,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.set_texture_wrap(texture, wrap_s, wrap_t) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_blit_render_target(
    handle: *mut Duct,
    src: DuctRenderTargetHandle,
    dst: DuctRenderTargetHandle,
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    filter: DuctFilterMode,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.blit_render_target(src, dst, src_w, src_h, dst_w, dst_h, filter) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_read_pixels(
    handle: *mut Duct,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    out_rgba: *mut u8,
    buf_len: usize,
) -> DuctResult {
    if handle.is_null() || out_rgba.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let slice = unsafe { std::slice::from_raw_parts_mut(out_rgba, buf_len) };
    match duct.read_pixels(x, y, width, height, slice) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_vsync(handle: *mut Duct, enabled: bool) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.set_vsync(enabled) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_set_uniform_int(
    handle: *mut Duct,
    name: *const c_char,
    val: i32,
) -> DuctResult {
    if handle.is_null() || name.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return DuctResult::ErrorInvalidParameter,
    };
    match duct.set_uniform_int(name_str, val) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_dispatch_compute(
    handle: *mut Duct,
    groups_x: u32,
    groups_y: u32,
    groups_z: u32,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.dispatch_compute(groups_x, groups_y, groups_z) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_draw_triangle(
    handle: *mut Duct,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    color: DuctColor,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.draw_triangle([x0, y0, 0.0], [x1, y1, 0.0], [x2, y2, 0.0], color) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_draw_rect(
    handle: *mut Duct,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: DuctColor,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.draw_rect(x, y, width, height, color) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[no_mangle]
pub extern "C" fn duct_draw_circle(
    handle: *mut Duct,
    center_x: f32,
    center_y: f32,
    radius: f32,
    segments: u32,
    color: DuctColor,
) -> DuctResult {
    if handle.is_null() { return DuctResult::ErrorInvalidParameter; }
    let duct = unsafe { &mut *handle };
    match duct.draw_circle(center_x, center_y, radius, segments, color) {
        Ok(()) => DuctResult::Success,
        Err(DuctError::PipelineOperationFailed(r)) => r,
        Err(_) => DuctResult::ErrorGeneric,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candidate_order_opengl() {
        let candidates = determine_load_candidates(BackendPreference::OpenGL);
        assert_eq!(candidates[0].0, BackendPreference::OpenGL);
    }

    #[test]
    fn test_candidate_order_dx12() {
        let candidates = determine_load_candidates(BackendPreference::DirectX12);
        assert_eq!(candidates[0].0, BackendPreference::DirectX12);
        assert_eq!(candidates[1].0, BackendPreference::OpenGL);
    }

    #[test]
    fn test_candidate_order_vulkan() {
        let candidates = determine_load_candidates(BackendPreference::Vulkan);
        assert_eq!(candidates[0].0, BackendPreference::Vulkan);
        assert_eq!(candidates[1].0, BackendPreference::OpenGL);
    }

    #[test]
    fn test_builder_window_required() {
        let builder = DuctBuilder::new();
        let res = builder.build();
        assert!(matches!(res, Err(DuctError::WindowRequired)));
    }
}
