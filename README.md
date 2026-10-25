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
- **Built-In Base Shape Helpers**: Immediate and buffered helpers for triangles (`draw_triangle`), rectangles (`draw_rect`), circles (`draw_circle`), and indexed quads (`create_rect_buffers`).
- **Live Shader Hot-Reloading**: Recompile and relink vertex, fragment, or compute shaders in-place at runtime (`duct.reload_shader(...)`) while preserving resource handles.
- **Real-Time GPU Telemetry**: Frame-by-frame FPS, frame time (ms), draw call count, triangle count, pipeline state switches, and tracked GPU memory allocations (`duct.frame_stats()`).
- **Low-Level Escape Hatches**: Query native GPU handles (`HGLRC`/`HDC`, `VkDevice`/`VkInstance`, `ID3D11Device`/`ID3D11DeviceContext`, `ID3D12Device`/`ID3D12CommandQueue`) via `duct.raw_context()`.
- **Multi-Language Bindings**: Ready-to-use C/C++ header ([`include/duct.h`](include/duct.h)), Python `ctypes` wrapper ([`bindings/python/duct.py`](bindings/python/duct.py)), and C# P/Invoke bindings ([`bindings/csharp/Duct.cs`](bindings/csharp/Duct.cs)).

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

## Running the Sandbox

The workspace includes an interactive sandbox ([`examples/sandbox.rs`](examples/sandbox.rs)) powered by [`windowed`](https://github.com/Retroboi64/windowed).

### Interactive Mode
```bash
# Launch with automatic backend detection
cargo run --example sandbox

# Launch with a specific backend
cargo run --example sandbox -- --gl
cargo run --example sandbox -- --vk
cargo run --example sandbox -- --dx11
cargo run --example sandbox -- --dx12
```
- Press **`B`** inside the window to live-switch between **OpenGL → Vulkan → DirectX 11 → DirectX 12** at runtime.
- Press **`ESC`** to exit cleanly.

### Automated Verification Mode (`--test`)
Runs hardware initialization, offscreen FBO rendering, GPU pixel readback verification, shader hot-reloading, and 10 rendered frames before exiting automatically:
```bash
cargo run --example sandbox -- --gl --test
cargo run --example sandbox -- --vk --test
cargo run --example sandbox -- --dx11 --test
cargo run --example sandbox -- --dx12 --test
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
