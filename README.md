# Duct — Modular Dynamic Graphics Subsystem (`Project32`)

**Duct** is a zero-dependency, cross-platform, dynamically loaded graphics rendering subsystem built in Rust with first-class C-ABI interoperability. Designed for the **Project32** engine ecosystem, Duct separates the frontend controller (`duct`) from runtime-loaded hardware rendering backends (`duct.gl`, `duct.vk`, `duct.dx11`, `duct.dx12`) via a strict C-ABI vtable (`DuctPipelineApi`).

---

## Key Features

- **Zero External Dependencies**: Written purely against the Rust standard library (`std`) with direct OS dynamic library loaders (`LoadLibraryW` / `GetProcAddress` on Windows, `dlopen` / `dlsym` on Linux & macOS). The only workspace dependency used for windowing integration and testing is [`windowed`](https://github.com/Retroboi64/windowed).
- **Strict Window Attachment**: Duct requires an attached native window handle (`HWND`, X11 `Window`/`Display*`, Wayland, or macOS `NSView*`) before initializing any graphics backend (`DuctError::WindowRequired`), with seamless `.with_windowed(&window)` ergonomics.
- **Hot-Swappable Hardware Backends**:
  - **OpenGL 3.3+ Core (`glduct`)**: Hardware WGL (Windows), GLX (Linux), and CGL (macOS) context creation, GLSL compilation, VAOs/VBOs/IBOs/UBOs, 2D textures, samplers, offscreen Framebuffer Objects (FBOs), blitting, pixel readback, instanced rendering, and compute dispatch.
  - **Vulkan 1.3 (`vkduct`)**: Hardware `vkCreateInstance`, physical device selection, swapchain (`VK_KHR_swapchain`), runtime SPIR-V 1.0 shader assembler, graphics & compute pipelines, descriptor sets, offscreen render passes, staging buffer pixel readback, and dynamic states.
  - **DirectX 11 (`dx11duct`)**: Hardware `D3D11CreateDeviceAndSwapChain` (Feature Level 11_0+), runtime HLSL compilation via `d3dcompiler_47.dll` (`vs_5_0`, `ps_5_0`, `cs_5_0`), input layouts, rasterizer/blend/depth-stencil states, constant buffers, 2D textures, offscreen RTVs/DSVs, and staging texture readback.
  - **DirectX 12 (`dx12duct`)**: Hardware `D3D12CreateDevice` (Feature Level 12_1) and `ID3D12CommandQueue` paired with DXGI swapchain presentation, runtime HLSL compilation, offscreen render targets, and hardware draw execution.
- **Live Runtime Backend Switching**: Switch between OpenGL, Vulkan, DirectX 11, and DirectX 12 on the fly on the same window (`duct.switch_backend(...)`) without restarting the application.
- **Built-In Base Shape Helpers**: Immediate and buffered helpers for triangles (`draw_triangle`), rectangles (`draw_rect`), circles (`draw_circle`), thick lines (`draw_line`), rectangle borders (`draw_rect_outline`), and circle rings (`draw_circle_outline`).
- **Flexible Backend Deployment**:
  - **Dynamic Plugins**: Separate backend `.dll` / `.so` libraries in `./renderer/` for hot-swappable modular engines.
  - **Direct Bundled Crates**: Statically link backends into your binary via features (`features = ["gl", "vk", "dx11", "dx12"]` or `features = ["bundled-all"]`) for zero-configuration crates.io consumption.
- **Live Shader Hot-Reloading**: Recompile and relink vertex, fragment, or compute shaders in-place at runtime (`duct.reload_shader(...)`) while preserving resource handles.
- **Real-Time GPU Telemetry**: Frame-by-frame FPS, frame time (ms), draw call count, triangle count, pipeline state switches, and tracked GPU memory allocations (`duct.frame_stats()`).
- **Low-Level Escape Hatches**: Query native GPU handles (`HGLRC`/`HDC`, `VkDevice`/`VkInstance`, `ID3D11Device`/`ID3D11DeviceContext`, `ID3D12Device`/`ID3D12CommandQueue`) via `duct.raw_context()`.
- **Multi-Language Bindings**: Ready-to-use C/C++ header ([`include/duct.h`](include/duct.h)), Python `ctypes` wrapper ([`bindings/python/duct.py`](bindings/python/duct.py)), and C# P/Invoke bindings ([`bindings/csharp/Duct.cs`](bindings/csharp/Duct.cs)).

---

## Backend Implementation Status & Honest Reality

> [!IMPORTANT]
> **Current Backend Reality**: **OpenGL (`glduct`) is currently the ONLY mature, fully working backend in Duct.**
> All 3D perspective pipelines, custom GLSL shaders, lighting models, procedural raymarching/effects, hardware depth testing, dynamic vertex layouts (positions, normals, UVs, colors), and offscreen FBOs with depth operate seamlessly on OpenGL.
>
> **DirectX 11 (`dx11duct`)**, **DirectX 12 (`dx12duct`)**, and **Vulkan (`vkduct`)** have severe issues and missing subsystem features in their current state. While they can open windows, clear backbuffers, and draw simple 2D shapes, they currently fail or fall back to minimal stubs when running real 3D meshes, custom shaders, or depth testing.

---

### Feature Matrix

| Feature | OpenGL (`glduct`) | DirectX 11 (`dx11duct`) | DirectX 12 (`dx12duct`) | Vulkan (`vkduct`) |
| :--- | :---: | :---: | :---: | :---: |
| **Current Usability Status** | 🟢 **Working / Mature** | 🔴 **Major Issues / 2D Only** | 🔴 **Major Issues / 2D Only** | 🟡 **Work in Progress** |
| **Window Surface & Swapchain Present** | ✅ Working | ✅ Working | ✅ Working | ✅ Working |
| **Live Backend Hot-Switching (`B` key)**| ✅ Working | ✅ Working | ✅ Working | ✅ Working |
| **2D Base Shapes (Tri, Rect, Circle, Line)**| ✅ Working | ✅ Working | ✅ Working | ✅ Working |
| **Custom Shaders (GLSL/HLSL/SPIR-V)** | ✅ Full Native GLSL | ❌ Discarded / Stub fallback | ❌ Discarded / Stub fallback | ❌ Built-in SPIR-V only |
| **Hardware Depth Testing & Depth Write** | ✅ Full D24/D32 buffer | ❌ **Broken (No DSV allocated)** | ❌ **Broken (No DSV allocated)** | 🟡 D32_SFLOAT (Swapchain/FBO) |
| **Dynamic Vertex Layouts (UV, Normal, etc.)** | ✅ Full VAO attributes | ❌ **Hardcoded to 1-2 attrs** | ❌ **Hardcoded to 1-2 attrs** | 🟡 Raw 8-attr float offset |
| **3D Perspective & Mesh Rendering** | ✅ Full MVP & Depth | ❌ Broken (No depth/layout) | ❌ Broken (No depth/layout) | 🟡 Fallback shader only |
| **Uniform Parameter Streaming** | ✅ Full `glUniform*` | 🟡 Fixed 96B ConstantBuffer | 🟡 Fixed 96B ConstantBuffer | 🟡 112B Push Constants |
| **Offscreen Render Targets (FBO/RTV)** | ✅ Full Color + Depth | 🟡 Color only (No DSV) | 🟡 Color only (No DSV) | 🟡 Color + Depth Pass |
| **Render Target Blitting to Window** | ✅ Hardware `glBlit` | 🟡 Incomplete | 🟡 Incomplete | 🟡 `vkCmdBlitImage` |
| **GPU Pixel Readback (Staging)** | ✅ `glReadPixels` | 🟡 Staging texture | 🟡 Readback buffer | 🟡 Staging buffer |
| **2D Textures & Samplers** | ✅ Working | 🟡 Basic default sampler | 🟡 Basic | 🟡 Basic default set |
| **Compute Shader Dispatch** | ✅ Working (GL 4.3+) | 🟡 Basic `cs_5_0` stub | 🟡 Basic stub | 🟡 Basic stub |
| **Shader Live Hot-Reloading** | ✅ Full Runtime Link | ❌ Replaces with stub | ❌ Replaces with stub | 🟡 Internal assembler only |
| **Telemetry & Frame Stats** | ✅ Working | ✅ Working | ✅ Working | ✅ Working |

---

### The Reality: What Works vs. What Has Issues

#### 🟢 OpenGL (`glduct`) — The Only Backend That Works Well
OpenGL 3.3 Core is the primary reference implementation in Duct and is the only backend where all examples run with full fidelity:
- **Full GLSL Compilation**: Compiles arbitrary GLSL 330 core vertex and fragment shaders at runtime with full driver error logging.
- **True Dynamic Vertex Layouts**: Generates exact VAO attribute pointer configurations for arbitrary combinations of positions (vec3), colors (vec3/vec4), normals (vec3), UVs (vec2), and custom strides.
- **Full Hardware Depth Testing**: Allocates real 24-bit/32-bit depth buffers for both swapchains and offscreen FBOs with configurable depth functions (`Less`, `LessEqual`, `Greater`, etc.).
- **Dynamic Uniform Reflection**: Queries and sets any uniform variable by name (`uMVP`, `uTime`, `uResolution`, `uMode`, light positions/colors).
- **All Examples Fully Functional**: `sandbox.rs`, `cube3d.rs`, `psx_retro.rs`, `lighting3d.rs`, and `shaders.rs` all render properly on OpenGL.

---

#### 🔴 DirectX 11 (`dx11duct`) — Critical Issues
While DirectX 11 initializes a swapchain and can draw flat 2D shapes, it fails for real 3D rendering and custom shaders:
1. **No Depth-Stencil Buffer (`ID3D11DepthStencilView`)**:
   - Neither the swapchain backbuffer nor offscreen render targets allocate a depth-stencil texture or view.
   - In `OMSetRenderTargets`, the 4th parameter (`pDepthStencilView`) is hardcoded to `NULL` (`std::ptr::null_mut()`).
   - Consequently, **3D depth testing is completely non-functional**, causing distant triangles to draw over front triangles.
2. **No Depth-Stencil State**:
   - `set_pipeline_state` creates rasterizer and blend states, but never allocates or binds an `ID3D11DepthStencilState`.
3. **Hardcoded Vertex Layouts**:
   - The input layout is hardcoded to either a 1-element layout (`POSITION`) or a 2-element layout (`POSITION` + `COLOR`).
   - Any mesh containing UV coordinates, normal vectors, or custom strides (e.g. 3D meshes) is either rejected by D3D11 or rendered with garbled vertex data.
4. **Custom Shaders Are Discarded**:
   - Duct does not contain a GLSL-to-HLSL cross-compiler. If shader code containing `#version` or `gl_Position` is passed, `dx11duct` discards the shader and silently substitutes a built-in 2D transform shader (`BUILTIN_VS_HLSL`) and flat color pixel shader (`BUILTIN_PS_HLSL`).
   - Procedural shaders, lighting shaders, and custom materials do not execute.

---

#### 🔴 DirectX 12 (`dx12duct`) — Critical Issues
Like DX11, the DirectX 12 backend is an early skeleton:
1. **No Depth-Stencil Buffer or DSV Heap**:
   - Does not allocate a DSV descriptor heap or depth texture; depth testing does not exist.
2. **Fixed 2D Root Signatures**:
   - Root signature and pipeline state objects only accept fixed 2D parameters; arbitrary uniforms and texture arrays fail.
3. **Hardcoded Input Layouts**:
   - Ignores custom vertex layouts; only maps basic 2D vertex positions.
4. **GLSL Shaders Discarded**:
   - Custom shader strings are replaced with built-in 2D bytecode stubs.

---

#### 🟡 Vulkan (`vkduct`) — Work in Progress
Vulkan has received swapchain depth buffers (`VK_FORMAT_D32_SFLOAT`), 8-attribute layout parsing, mid-frame render pass switching, and 112-byte push constant streaming, but still has fundamental limitations:
1. **No Runtime GLSL Compiler**:
   - Vulkan requires pre-compiled binary SPIR-V bytecode. Because Duct has zero external dependencies and does not bundle `glslang` or `shaderc`, raw GLSL strings cannot be compiled to SPIR-V at runtime.
   - When GLSL source is passed, `vkduct` falls back to internal pre-assembled SPIR-V bytecode that only renders vertex colors with a transform matrix. Custom fragment shaders (such as procedural raymarching, CRT simulation, or lighting shaders) cannot run without precompiled SPIR-V words.
2. **Minimal Descriptor Set Architecture**:
   - Uses push constants for uniforms; dynamic descriptor set pools for multi-texture binding and structured storage buffers (SSBOs) are minimal.

---

### What Needs to Be Done for Parity
To bring Vulkan, DX11, and DX12 up to the level of OpenGL:
- **Shader Pipeline**: Integrate a lightweight shader cross-compiler (e.g. Naga or SPIRV-Cross) or establish a unified bytecode format across all backends.
- **DirectX 11 & 12**: Allocate real depth-stencil textures and views (`ID3D11DepthStencilView` / D3D12 DSV descriptor heap), bind `ID3D11DepthStencilState`, and dynamically construct input element layouts from `DuctVertexLayout`.

---

## Architecture & Workspace Layout

```text
duct/
├── Cargo.toml                 # Workspace manifest & root sandbox example
├── build_dist.rs              # Root build wrapper delegating to xtask
├── xtask/                     # Build & distribution orchestrator (`cargo xtask`)
├── crates/
│   ├── duct-core/             # Shared C-ABI types, handles, and DuctPipelineApi vtable
│   ├── duct/                  # Main engine router (`duct.dll` / `libduct.so` / `libduct.dylib`)
│   ├── glduct/                # OpenGL 3.3 Core backend (`renderer/duct.gl.*`)
│   ├── vkduct/                # Vulkan 1.3 backend (`renderer/duct.vk.*`)
│   ├── dx11duct/              # DirectX 11 backend (`renderer/duct.dx11.*`)
│   └── dx12duct/              # DirectX 12 backend (`renderer/duct.dx12.*`)
├── examples/
│   └── sandbox.rs             # Interactive & automated test sandbox using `windowed`
├── include/
│   └── duct.h                 # C99 / C11 / C++ FFI header
└── bindings/
    ├── csharp/Duct.cs         # C# (.NET) P/Invoke bindings
    └── python/                # Python ctypes wrapper & test suite
        ├── duct.py
        └── test_duct.py
```

---

## Building & Deployment

Duct uses the `cargo xtask` pattern to compile the core library and backend plugins, automatically renaming and staging compiled backend shared libraries into `./renderer/` and `./target/<profile>/renderer/`.

### Build All Backends (Debug)
```bash
cargo xtask build --all
```

### Build Specific Backends Only
```bash
# Build only OpenGL and Vulkan
cargo xtask build --only gl,vk

# Build only DirectX 11 and DirectX 12
cargo xtask build --only dx11,dx12
```

### Build Optimized Release Distribution (`./dist/`)
```bash
cargo xtask build --all --release --dist
```
This packages the release binaries, `renderer/` backend modules, C headers, and language bindings into `./dist/`.

---

## Running the Examples

Duct includes a suite of interactive examples powered by [`windowed`](https://github.com/Retroboi64/windowed) demonstrating 2D shapes, 3D perspective, retro PSX graphics, real-time lighting, and procedural shaders across all 4 backends (OpenGL, Vulkan, DirectX 11, DirectX 12).

### 1. `examples/sandbox.rs` — 2D Base Shapes & FBO Sandbox
Comprehensive testbed for window attachment, 2D primitives (`draw_triangle`, `draw_rect`, `draw_circle`, `draw_line`), FBO offscreen rendering, pixel readback, telemetry, and live backend hot-switching (`B` key).
```bash
cargo run --example sandbox -- --gl
cargo run --example sandbox -- --vk
cargo run --example sandbox -- --dx11
cargo run --example sandbox -- --dx12
```

### 2. `examples/cube3d.rs` — 3D Perspective Cube
Full 3D pipeline demonstration featuring zero-dependency column-major `Mat4` (perspective projection, camera look-at, model rotation) and depth testing.
```bash
cargo run --example cube3d -- --gl
cargo run --example cube3d -- --vk
cargo run --example cube3d -- --dx11
cargo run --example cube3d -- --dx12
```

### 3. `examples/psx_retro.rs` — PSX Retro Graphics Simulation
Authentic 90s console rendering pipeline with:
- Fixed-point vertex grid snapping & wobble/jitter in NDC clip space.
- Low-resolution rasterization to a 320×240 offscreen Render Target (FBO).
- Fullscreen nearest-neighbor upscale blit to the window.
- Atmospheric distance depth fog.
- Rotating low-poly gem and satellite primitives.
```bash
cargo run --example psx_retro -- --gl
cargo run --example psx_retro -- --vk
cargo run --example psx_retro -- --dx11
cargo run --example psx_retro -- --dx12
```

### 4. `examples/lighting3d.rs` — 3D Real-Time Blinn-Phong Lighting
Procedural UV sphere (24 rings × 32 sectors) with full vertex normals and dynamic multi-light Blinn-Phong shading:
- Ambient environmental lighting.
- Directional sunlight with diffuse cosine Lambertian falloff.
- Orbiting point light with quadratic distance attenuation.
- Specular specular gloss reflections and camera specular highlights.
```bash
cargo run --example lighting3d -- --gl
cargo run --example lighting3d -- --vk
cargo run --example lighting3d -- --dx11
cargo run --example lighting3d -- --dx12
```

### 5. `examples/shaders.rs` — Advanced Procedural Shader Playground
Interactive full-screen procedural shader lab with dynamic uniform buffers and live hot-reloading:
- **Mode 1**: Cyberpunk Neon Plasma / Interference Warp.
- **Mode 2**: Retro CRT Monitor (barrel curvature, horizontal scanlines, phosphor glow, RGB chromatic aberration split).
- **Mode 3**: Real-Time 3D Raymarched Gyroid / SDF Minimal Surface with normal estimation and iridescent shading.
- **Mode 4**: Water Ripple & Underwater Caustics.
- **Controls**: Keys `[1]`, `[2]`, `[3]`, `[4]` to switch effects, `[R]` to live hot-reload fragment shader, `[Space]` to pause/resume time, `[B]` to switch backends.
```bash
cargo run --example shaders -- --gl
cargo run --example shaders -- --vk
cargo run --example shaders -- --dx11
cargo run --example shaders -- --dx12
```

### Automated Verification Mode (`--test`)
All examples support the `--test` flag for headless/automated CI testing (renders 10 frames and exits with code 0):
```bash
cargo run --example sandbox -- --gl --test
cargo run --example cube3d -- --vk --test
cargo run --example psx_retro -- --dx11 --test
cargo run --example lighting3d -- --dx12 --test
cargo run --example shaders -- --gl --test
```

---

## Rust Usage Example

```rust
use duct::{BackendPreference, Duct, DuctColor};
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a native window via `windowed`
    let mut window = Window::new(
        WindowConfig::new("Project32 — Duct App")
            .size(1280, 720)
            .resizable(true),
    )?;

    // 2. Attach the window to Duct and initialize from CLI args (--gl, --vk, --dx11, --dx12)
    let mut duct = Duct::builder()
        .with_windowed(&window)
        .from_args()
        .vsync(true)
        .build()?;

    println!("Active Backend: {}", duct.backend_name());

    // 3. Run the render loop
    window.run(move |event, _win| {
        match event {
            Event::CloseRequested | Event::KeyDown { key: Key::Escape, .. } => {
                return ControlFlow::Exit;
            }
            Event::KeyDown { key: Key::B, .. } => {
                // Live switch backend at runtime
                let _ = duct.switch_backend(BackendPreference::Vulkan);
            }
            Event::Resized { width, height } => {
                let _ = duct.resize_viewport(width, height);
            }
            Event::RedrawRequested => {
                let _ = duct.begin_frame(DuctColor::CORNFLOWER_BLUE);

                // Draw base shapes directly on the GPU
                let _ = duct.draw_triangle(
                    [0.0, 0.5, 0.0],
                    [-0.5, -0.5, 0.0],
                    [0.5, -0.5, 0.0],
                    DuctColor::rgba(1.0, 0.55, 0.1, 1.0),
                );
                let _ = duct.draw_rect(-0.85, 0.45, 0.3, 0.3, DuctColor::GREEN);
                let _ = duct.draw_circle(0.7, 0.6, 0.18, 36, DuctColor::WHITE);

                let _ = duct.end_frame();
                let _ = duct.present();
            }
            _ => {}
        }
        ControlFlow::Poll
    })?;

    Ok(())
}
```

---

## C / Python / C# FFI Usage

### Python (`bindings/python/duct.py`)
```python
from duct import Duct

duct = Duct.from_args(hwnd=my_hwnd)
duct.begin_frame(0.1, 0.1, 0.2, 1.0)
duct.draw_triangle(0.0, 0.5, -0.5, -0.5, 0.5, -0.5, 1.0, 0.5, 0.2, 1.0)
duct.draw_rect(-0.8, 0.4, 0.3, 0.3, 0.2, 0.85, 0.4, 1.0)
duct.draw_circle(0.6, 0.5, 0.2, 32, 0.2, 0.7, 1.0, 1.0)
duct.end_frame()
duct.present()

# Live switch to DirectX 11 (5), DirectX 12 (2), Vulkan (3), or OpenGL (1)
duct.switch_backend(5)
```

Run the automated Python FFI test suite:
```bash
python bindings/python/test_duct.py
```

### C / C++ (`include/duct.h`)
```c
#include "duct.h"

DuctWindowHandle win = duct_window_from_win32(hwnd, hinstance);
DuctHandle duct = NULL;
if (duct_create_from_args(win, &duct) == DUCT_SUCCESS) {
    DuctColor bg = { 0.1f, 0.12f, 0.2f, 1.0f };
    DuctColor orange = { 1.0f, 0.55f, 0.1f, 1.0f };

    duct_begin_frame(duct, bg);
    duct_draw_triangle(duct, 0.0f, 0.5f, -0.5f, -0.5f, 0.5f, -0.5f, orange);
    duct_end_frame(duct);
    duct_present(duct);

    duct_destroy(duct);
}
```

---

## Running Tests

```bash
# Run Rust unit tests across all crates
cargo test --workspace

# Run Python FFI & live backend switching tests
python bindings/python/test_duct.py
```

---

## License

Dual-licensed under **MIT** or **Apache-2.0**.
