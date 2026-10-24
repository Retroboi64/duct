# Project32 Graphics Subsystem: Build, Architecture & Module Guide

The `duct` graphics subsystem provides an ABI-safe, modular, dynamic rendering pipeline architecture for Project32. Built on a "one crate at a time" philosophy prioritizing simplicity, control, and zero bloat, `duct` decouples backend rendering implementations from engine logic using C-ABI dynamic libraries (`.dll` on Windows, `.so` on Linux, `.dylib` on macOS).

---

## Table of Contents
1. [Architecture Overview](#1-architecture-overview)
2. [Native Window Requirement & `windowed` Integration](#2-native-window-requirement--windowed-integration)
3. [Full Real OpenGL GPU Pipeline (`glduct`)](#3-full-real-opengl-gpu-pipeline-glduct)
4. [Extended Graphics Pipeline Functions](#4-extended-graphics-pipeline-functions)
5. [Offscreen Render Targets & Render-to-Texture](#5-offscreen-render-targets--render-to-texture)
6. [Flexible Vertex Layouts & Instanced Drawing](#6-flexible-vertex-layouts--instanced-drawing)
7. [Shader Hot-Reloading & Live Program Cache](#7-shader-hot-reloading--live-program-cache)
8. [Real-time Telemetry & Frame Profiler](#8-real-time-telemetry--frame-profiler)
9. [Live Dynamic Backend Switching](#9-live-dynamic-backend-switching)
10. [Ergonomic Typed GPU Resources](#10-ergonomic-typed-gpu-resources)
11. [Multi-Language Support (C, C++, Python, C#)](#11-multi-language-support-c-c-python-c)
12. [CLI Arguments & Dynamic Routing](#12-cli-arguments--dynamic-routing)
13. [Hot-Removal & Graceful Fallback](#13-hot-removal--graceful-fallback)
14. [Custom Build Settings (xtask)](#14-custom-build-settings-xtask)
15. [Stripping / Removing Unused Modules](#15-stripping--removing-unused-modules)
16. [Adding a New Pipeline Backend](#16-adding-a-new-pipeline-backend)
17. [Low-Level Escape Hatches](#17-low-level-escape-hatches)

---

## 1. Architecture Overview

```text
                      +-----------------------------+
                      |       Project32 Engine      |
                      |   (or sandbox.exe / Game)   |
                      +--------------+--------------+
                                     |
                          Interacts directly with
                                     |
                                     v
                      +-----------------------------+
                      |         crates/duct         |
                      |   Central Router & Loader   |
                      +--------------+--------------+
                                     |
                   Dynamic C-ABI loading (Native OS loader: LoadLibraryW / dlopen)
                    Discovers ./renderer/duct.*.dll
                                     |
        +----------------------------+----------------------------+
        |                                                         |
        v                                                         v
+---------------+  (if missing, fallback)  +----------------+  (fallback)  +---------------+
|    glduct     | -----------------------> |    vkduct      | -----------> |   dx12duct    |
| (duct.gl.dll) |                          | (duct.vk.dll)  |              | (duct.dx12.dll|
+---------------+                          +----------------+              +---------------+
```

---

## 2. Native Window Requirement & `windowed` Integration

To guarantee safe surface connection and prevent driver panics, **`duct` strictly requires an attached native window** during initialization:

```rust
// Attempting to build without an attached window returns an error:
let result = Duct::builder().from_args().build();
assert!(matches!(result, Err(DuctError::WindowRequired)));
```

### First-Class `windowed` Support
`duct` natively integrates with [`windowed`](https://github.com/Retroboi64/windowed):

```rust
use windowed::{Window, WindowConfig};
use duct::Duct;

let window = Window::new(WindowConfig::new("Project32").size(1280, 720))?;

// Attach the window directly:
let mut duct = Duct::builder()
    .with_windowed(&window)
    .from_args()
    .build()?;
```

### Cross-Platform Native Window Handles
`duct` extracts native handles seamlessly from `windowed::Window` using its `rwd` module:
- **Windows**: `Win32WindowHandle` (`HWND`, `HINSTANCE`)
- **Linux**: `XlibWindowHandle` (`Window` XID, `Display*`)
- **macOS**: `AppKitWindowHandle` (`NSView*`)

When using custom window managers or foreign languages (C, C++, Python, C#), use the dedicated handle constructors:

```rust
// Idiomatic Rust constructors:
let win_handle  = DuctWindowHandle::from_win32(hwnd, hinstance);
let linux_handle = DuctWindowHandle::from_xlib(xid, display_ptr);
let mac_handle   = DuctWindowHandle::from_appkit(ns_view_ptr);
```

```c
// C / C++ helper functions (include/duct.h):
DuctWindowHandle win_h   = duct_window_from_win32(hwnd, hinstance);
DuctWindowHandle linux_h = duct_window_from_xlib(xid, display);
DuctWindowHandle mac_h   = duct_window_from_appkit(ns_view);
```

---

## 3. Full Real OpenGL GPU Pipeline (`glduct`) Across Platforms

`glduct` (`duct.gl.dll` / `duct.gl.so` / `duct.gl.dylib`) executes real hardware OpenGL rendering on the GPU with 100% zero external dependencies across all major operating systems:

### Windows (Win32 & WGL)
- **WGL & Win32 Integration**: Binds `HWND`, retrieves `HDC`, sets hardware `PIXELFORMATDESCRIPTOR`, initializes WGL context, and calls `SwapBuffers(hdc)`.
- **Proc Loader**: Resolves core functions through `opengl32.dll` and modern extensions via `wglGetProcAddress`.

### Linux (X11 & GLX)
- **Xlib & GLX Integration**: Binds `Window` XID and X11 `Display*`, resolves visuals via `glXChooseVisual`, creates contexts via `glXCreateContext`, and presents via `glXSwapBuffers(display, window)`.
- **Proc Loader**: Dynamically discovers modern OpenGL core extensions via `glXGetProcAddressARB` / `glXGetProcAddress` and falls back to `libGL.so.1` / `libGL.so`.
- **Seamless Context Sharing**: Detects if an active OpenGL context was already initialized by `windowed` or host engine and binds without duplicate creation.

### macOS (Cocoa, AppKit & CGL)
- **CGL & AppKit Integration**: Binds `NSView*`, requests Core Profile 3.2+ capabilities with double-buffering via `CGLChoosePixelFormat` / `CGLCreateContext` (or adopts active Cocoa `NSOpenGLContext`), and presents via `CGLFlushDrawable(ctx)`.
- **Proc Loader**: Resolves symbols dynamically from `/System/Library/Frameworks/OpenGL.framework/OpenGL` via `dlopen`/`dlsym`.

### Universal Core Features
- **VAO & Core Profile**: Allocates and maintains default Vertex Array Objects for modern Core Profile compliance.
- **Real GPU Compiles**: Real runtime GLSL vertex/fragment compilation with error log inspection.
- **VBO / EBO Allocations**: Hardware buffer allocation via `glGenBuffers`, `glBufferData`, and `glBufferSubData`.
- **Hardware Draw**: Issues real `glDrawArrays` and `glDrawElements` calls.

---

## 4. Extended Graphics Pipeline Functions

`duct` and `duct-core` expose a complete suite of graphics operations:

| Function | Description |
| :--- | :--- |
| `resize_viewport(width, height)` | Resizes the client rendering viewport (`glViewport`) |
| `set_scissor(x, y, width, height)` | Restricts drawing to a scissor box (`glScissor`) |
| `begin_frame(clear_color)` | Clears color, depth, and stencil buffers |
| `end_frame()` | Closes frame boundaries and tracks render stats |
| `present()` | Swaps buffers to display the rendered frame |
| `create_shader(stage, src, entry)` | Compiles vertex, fragment, or compute shaders |
| `destroy_shader(handle)` | Deallocates shader resources on the GPU |
| `reload_shader(handle, stage, src, entry)` | Hot-reloads shader and flushes pipeline program cache |
| `create_buffer(type, size, data, dynamic)` | Allocates VBO, EBO, UBO, or SSBO buffers |
| `update_buffer(handle, offset, data)` | Updates existing buffer memory (`glBufferSubData`) |
| `destroy_buffer(handle)` | Deallocates buffer resources on the GPU |
| `create_texture_2d(w, h, format, data)` | Uploads 2D textures (RGBA8, RGB8, R8) |
| `generate_mipmaps(handle)` | Generates complete mipmap chain (`glGenerateMipmap`) |
| `set_texture_filter(handle, filter)` | Configures texture min/mag filtering & mipmap sampling |
| `bind_texture(slot, handle)` | Binds texture to a multi-texturing unit slot |
| `destroy_texture(handle)` | Deallocates texture resources |
| `create_render_target(desc)` | Creates offscreen FBO with color & depth attachments |
| `destroy_render_target(handle)` | Releases FBO, color texture, and depth renderbuffer |
| `bind_render_target(handle)` | Diverts draw calls to FBO (`handle=0` restores swapchain) |
| `get_render_target_texture(handle)` | Returns texture handle to sample render target in shaders |
| `set_pipeline_state(desc)` | Links shaders, vertex layout, depth comparison, blend, cull |
| `set_wireframe(enabled)` | Toggles polygon rasterization mode (`glPolygonMode`) |
| `set_uniform_float / vec2 / vec3 / vec4 / mat4` | Updates shader uniform parameters |
| `draw(vertex_count, first_vertex)` | Non-indexed triangle draw (`glDrawArrays`) |
| `draw_indexed(index_count, first_index)`| Indexed triangle draw (`glDrawElements`) |
| `draw_instanced(vertices, instances, first_vertex)` | Instanced triangle draw (`glDrawArraysInstanced`) |
| `draw_indexed_instanced(indices, instances, ...)` | Indexed instanced draw (`glDrawElementsInstanced`) |
| `frame_stats()` | Queries real-time frame telemetry and smoothed FPS |
| `switch_backend(preference)` | Live runtime backend switching without closing window |
| `raw_gl_context()` | Low-level escape hatch (HGLRC, HDC, proc loader) |

---

## 5. Offscreen Render Targets & Render-to-Texture

`duct` features first-class support for offscreen render targets (Frame Buffer Objects), essential for post-processing bloom, shadow mapping, reflections, UI overlays, and deferred lighting passes.

```rust
// Create an offscreen render target (1280x720 with depth buffer)
let fbo = duct.create_render_target(&DuctRenderTargetDesc::new(1280, 720).with_depth())?;

// Direct rendering to the offscreen texture
duct.bind_render_target(fbo)?;
duct.begin_frame(DuctClearColor::new(0.2, 0.0, 0.4, 1.0))?;
// ... draw 3D scene into offscreen target ...
duct.end_frame()?;

// Restore default backbuffer / swapchain
duct.bind_render_target(DuctRenderTargetHandle::BACKBUFFER)?;

// Sample the offscreen texture in a post-process pass
let tex_handle = duct.get_render_target_texture(fbo)?;
duct.bind_texture(0, tex_handle)?;
// ... render screen-aligned quad with post-process shader ...

// Clean up FBO resources when finished
duct.destroy_render_target(fbo)?;
```

---

## 6. Flexible Vertex Layouts & Instanced Drawing

### Declarative Vertex Layouts
Define interleaved or multi-attribute vertex buffers easily with builder semantics:

```rust
let layout = DuctVertexLayout::new(32) // 32-byte stride (Position + Normal + UV)
    .with_attribute(DuctVertexAttribute::new(0, DuctVertexFormat::Float3, 0))   // pos
    .with_attribute(DuctVertexAttribute::new(1, DuctVertexFormat::Float3, 12))  // normal
    .with_attribute(DuctVertexAttribute::new(2, DuctVertexFormat::Float2, 24)); // uv

let pipeline_desc = DuctPipelineDesc::new(vs, fs)
    .with_layout(layout)
    .with_depth_test(true, true, DuctCompareOp::LessEqual)
    .with_blend_mode(DuctBlendMode::AlphaBlend)
    .with_cull_mode(DuctCullMode::Back);

duct.set_pipeline_state(&pipeline_desc)?;
```

### Instanced Rendering
Render thousands of particles, foliage, or batch instances in a single draw call:

```rust
// Draw 500 instances of a 36-vertex cube
duct.draw_instanced(36, 500, 0)?;

// Draw 1000 indexed mesh instances
duct.draw_indexed_instanced(indices.len() as u32, 1000, 0, 0)?;
```

---

## 7. Shader Hot-Reloading & Live Program Cache

Iterate on shaders live without restarting your application. `reload_shader` recompiles the GLSL or SPIR-V/DXBC bytecode and automatically flushes the internal linked program cache:

```rust
let new_fs_src = std::fs::read_to_string("assets/shaders/toon.frag")?;
duct.reload_shader(fs_handle, DuctShaderStage::Fragment, &new_fs_src, "main")?;
// Next draw call automatically links and uses the newly compiled shader!
```

---

## 8. Real-time Telemetry & Frame Profiler

Track rendering performance and draw call budgets with `frame_stats()`:

```rust
let stats = duct.frame_stats();
println!("FPS: {:.1} (Frame Time: {:.2} ms)", stats.smoothed_fps, stats.frame_time_ms);
println!("Frame: {}, Draw Calls: {}, Triangles: {}", stats.frame_count, stats.draw_calls, stats.triangles);
```

---

## 9. Live Dynamic Backend Switching

Switch backends dynamically at runtime without recreating the native OS window or losing window state:

```rust
// Live hot-swap to DirectX 12 or Vulkan (e.g. on keypress 'B')
duct.switch_backend(BackendPreference::DirectX12)?;
// The engine flushes the previous pipeline, binds the existing HWND/XID/NSView
// to the new backend, and resumes rendering seamlessly!
```

---

## 10. Ergonomic Typed GPU Resources

For modern idiomatic Rust codebases, `duct` provides high-level typed RAII wrappers around raw handles:

```rust
// RAII GpuBuffer
let vbo = duct.create_gpu_buffer(
    DuctBufferType::VertexBuffer,
    std::mem::size_of_val(&vertices),
    Some(vertices.as_ptr() as *const _),
    false,
)?;
vbo.bind(&mut duct, 0)?;

// RAII GpuRenderTarget
let rt = duct.create_gpu_render_target(1920, 1080, true)?;
rt.bind(&mut duct)?;
let tex = rt.color_texture();
```

---

## 11. Multi-Language Support (C, C++, Python, C#)

`duct.dll` exports standard C-ABI functions allowing non-Rust applications to leverage the pipeline.

- **C / C++**: Include [`include/duct.h`](file:///c:/Users/reese/Desktop/duct/include/duct.h).
- **Python**: Import [`bindings/python/duct.py`](file:///c:/Users/reese/Desktop/duct/bindings/python/duct.py) (uses standard `ctypes`).
- **C# / .NET**: Import [`bindings/csharp/Duct.cs`](file:///c:/Users/reese/Desktop/duct/bindings/csharp/Duct.cs) (P/Invoke).
- Full documentation in [`docs/LANGUAGE_BINDINGS.md`](file:///c:/Users/reese/Desktop/duct/docs/LANGUAGE_BINDINGS.md).

---

## 12. CLI Arguments & Dynamic Routing

| Flag | Target Backend | Windows (.dll) | Linux (.so) | macOS (.dylib) |
| :--- | :--- | :--- | :--- | :--- |
| `--gl` or `--opengl` | OpenGL Core | `./renderer/duct.gl.dll` | `./renderer/duct.gl.so` | `./renderer/duct.gl.dylib` |
| `--dx12` or `--d3d12` | DirectX 12 | `./renderer/duct.dx12.dll`| *(Windows Only)* | *(Windows Only)* |
| `--vk` or `--vulkan` | Vulkan | `./renderer/duct.vk.dll` | `./renderer/duct.vk.so` | `./renderer/duct.vk.dylib` |
| `--renderer-dir <PATH>`| Custom Search Path | `<PATH>/duct.*.dll` | `<PATH>/duct.*.so` | `<PATH>/duct.*.dylib` |

Run the sandbox example:
```bash
cargo run --example sandbox -- --gl
cargo run --example sandbox -- --dx12   # (Windows only, falls back to GL on Linux/macOS)
cargo run --example sandbox -- --vk
cargo run --example sandbox -- --test
```

---

## 13. Hot-Removal & Graceful Fallback

If `duct.dx12.dll` is deleted from `./renderer/` because a user only wants OpenGL, the engine warns and falls back to `duct.gl.dll` without crashing.

On Linux and macOS, if `--dx12` is requested, `duct` identifies that DirectX 12 is unsupported on Unix, emits a clean log warning, and seamlessly falls back to OpenGL or Vulkan.

---

## 14. Custom Build Settings (xtask)

```bash
# Build all pipeline modules for the current OS
cargo xtask build --all

# Build only specific backends
cargo xtask build --only gl
cargo xtask build --only dx12
cargo xtask build --only vk

# Release builds
cargo xtask build --all --release

# Clean renderer directories
cargo xtask clean
```

Artifacts are automatically named with the appropriate platform suffix and deployed into both `./renderer/` and `./target/<profile>/renderer/`:
- **Windows**: `duct.gl.dll`, `duct.dx12.dll`, `duct.vk.dll`, `duct.dll`
- **Linux**: `duct.gl.so` (and `libduct.gl.so`), `duct.vk.so`, `libduct.so`
- **macOS**: `duct.gl.dylib` (and `libduct.gl.dylib`), `duct.vk.dylib`, `libduct.dylib`

---

## 15. Stripping / Removing Unused Modules

Shipping games can reduce download and installation size without recompiling the game binary:
- **OpenGL-only build**: Simply delete `duct.dx12.*` and `duct.vk.*` from the `./renderer/` directory.
- **Vulkan-only build**: Simply delete `duct.gl.*` and `duct.dx12.*`.
- When an end user launches the executable, `duct` scans the directory, sees only the remaining DLLs/SOs, and initializes the available pipeline seamlessly.

---

## 16. Adding a New Pipeline Backend

To add a new backend (e.g. `metalduct` or `software_duct`):

1. **Create crate** in `crates/mybackend/`:
   ```toml
   [package]
   name = "mybackend"
   version = "0.1.0"
   edition = "2024"

   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   duct-core = { path = "../duct-core" }
   ```

2. **Implement `RenderPipeline`** from `duct-core`:
   ```rust
   use duct_core::*;

   pub struct MyPipeline;
   impl RenderPipeline for MyPipeline {
       fn backend_type(&self) -> DuctBackendType { DuctBackendType::Custom }
       fn name(&self) -> &str { "Custom Backend" }
       fn version(&self) -> &str { "0.1.0" }
       fn init(&mut self, config: &DuctInitConfig) -> DuctResult {
           if !config.window.is_valid() { return DuctResult::ErrorWindowRequired; }
           DuctResult::Success
       }
       // ... implement lifecycle and draw methods ...
   }

   export_pipeline!(MyPipeline);
   ```

3. **Deploy to `./renderer/`**:
   Name the output `duct.custom.dll` (or `.so` / `.dylib`), place it in `./renderer/`, and `duct` will discover and load it dynamically!

---

## 17. Low-Level Escape Hatches

When you need direct control over backend API state:

```rust
// Retrieve raw OpenGL context pointers across platforms:
if let Some(gl) = duct.raw_gl_context() {
    // Windows: HGLRC (hglrc), HDC (hdc)
    // Linux: GLXContext (hglrc), Display* (hdc)
    // macOS: CGLContextObj (hglrc)
    println!("GL Context: {:?}", gl.hglrc);
    println!("Device/Display: {:?}", gl.hdc);

    // Call native OpenGL functions via proc loader:
    if let Some(loader) = gl.get_proc_address {
        let gl_flush = unsafe { loader(b"glFlush\0".as_ptr() as *const _) };
        // ...
    }
}

// Or send custom vendor commands directly:
duct.execute_custom(1, std::ptr::null_mut())?;
```
