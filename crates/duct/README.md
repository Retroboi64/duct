# duct

**Duct** is a zero-dependency, cross-platform, modular graphics rendering crate built in Rust for the Project32 engine ecosystem.

Duct provides a high-level, type-safe API for hardware-accelerated rendering across **OpenGL 3.3+**, **Vulkan 1.3**, **DirectX 11**, and **DirectX 12** on Windows, Linux, and macOS.

## Highlights
- **Zero External Dependencies**: Implemented purely using the Rust standard library and raw OS bindings (with optional `windowed` windowing support).
- **Window Attachment Requirement**: Enforces native window attachment before pipeline initialization.
- **Dynamic Plugin & Modular Architecture**: Supports both runtime-loaded dynamic backend plugins (`duct.gl`, `duct.vk`, `duct.dx11`, `duct.dx12`) and static backend linking.
- **Live Runtime Backend Switching**: Seamlessly transition between graphics backends on the same window without restarting.
- **Base Shape Helpers**: Built-in immediate and buffered shape rendering (`draw_triangle`, `draw_rect`, `draw_circle`, `create_rect_buffers`).
- **Live Shader Hot-Reloading**: In-place recompilation of shaders at runtime.
- **GPU Telemetry**: Real-time FPS, frame timing, draw calls, triangle counts, and memory statistics.
- **C-ABI & Multi-Language FFI**: Direct C headers, Python `ctypes`, and C# bindings.

## Quickstart

```rust
use duct::{Duct, DuctColor, BackendPreference};
use windowed::{Window, WindowConfig, Event, ControlFlow, Key};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new(WindowConfig::new("Duct Window").size(1280, 720))?;

    let mut duct = Duct::builder()
        .with_windowed(&window)
        .from_args()
        .build()?;

    window.run(move |event, _win| {
        match event {
            Event::CloseRequested => ControlFlow::Exit,
            Event::RedrawRequested => {
                let _ = duct.begin_frame(DuctColor::CORNFLOWER_BLUE);
                let _ = duct.draw_rect(-0.5, -0.5, 1.0, 1.0, DuctColor::GREEN);
                let _ = duct.end_frame();
                let _ = duct.present();
            }
            _ => ControlFlow::Poll,
        }
    })?;

    Ok(())
}
```

## License
Dual-licensed under MIT or Apache-2.0.
