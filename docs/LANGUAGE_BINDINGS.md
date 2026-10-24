# Project32 Duct: Multi-Language Bindings & FFI Guide

`duct` compiles to `duct.dll` (or `libduct.so` / `libduct.dylib`) exporting pure C-ABI symbols. This enables any programming language with C FFI support (C, C++, C#, Python, Zig, Go, Nim, etc.) to drive the graphics subsystem without touching Rust.

---

## 1. C & C++ Integration

Include [`include/duct.h`](file:///c:/Users/reese/Desktop/duct/include/duct.h) and link against `duct.lib` (or load dynamically with `LoadLibrary` / `dlopen`).

```c
#include "duct.h"

int main() {
    DuctWindowHandle window;
    window.native_window = (void*)my_hwnd;
    window.native_display = NULL;

    DuctInitConfig cfg = {
        .window = window,
        .width = 1280,
        .height = 720,
        .vsync = true,
        .debug_mode = true,
        .app_name = "My C Game"
    };

    DuctHandle duct = NULL;
    DuctResult res = duct_create(&cfg, &duct);
    if (res != DUCT_SUCCESS) {
        // Handle error (e.g. DUCT_ERROR_WINDOW_REQUIRED)
        return -1;
    }

    DuctColor bg = { 0.1f, 0.2f, 0.3f, 1.0f };
    duct_begin_frame(duct, bg);
    
    // Draw instanced particles or geometry
    duct_draw_instanced(duct, 3, 100, 0);
    
    duct_end_frame(duct);
    duct_present(duct);

    // Query frame telemetry and profiler stats
    DuctFrameStats stats;
    if (duct_get_frame_stats(duct, &stats) == DUCT_SUCCESS) {
        printf("FPS: %.1f, DrawCalls: %u, Tris: %llu\n", stats.smoothed_fps, stats.draw_calls, stats.triangles_rendered);
    }

    // Dynamic runtime live backend switching (e.g. switch to Vulkan)
    duct_switch_backend(duct, DUCT_BACKEND_VULKAN);

    duct_destroy(duct);
    return 0;
}
```

---

## 2. Python Integration (ctypes)

The [`bindings/python/duct.py`](file:///c:/Users/reese/Desktop/duct/bindings/python/duct.py) module wraps `duct.dll` with standard library `ctypes` (zero dependencies required):

```python
from duct import Duct

# Initialize with native window handle (HWND on Windows)
duct = Duct(hwnd=my_hwnd, width=1280, height=720)
print("Active backend:", duct.backend_name)

# Offscreen render target (FBO)
rt = duct.create_render_target(800, 600, has_depth=True)
duct.bind_render_target(rt)
duct.begin_frame(0.2, 0.0, 0.4, 1.0)
duct.draw(3, 0)
duct.end_frame()

# Draw to swapchain with instancing
duct.bind_render_target(0)
duct.begin_frame(0.39, 0.58, 0.93, 1.0)
duct.draw_instanced(3, 500)
duct.end_frame()
duct.present()

# Telemetry
stats = duct.get_frame_stats()
print(f"FPS: {stats['fps']:.1f}, DrawCalls: {stats['draw_calls']}")

# Live switch to Vulkan
duct.switch_backend(3)
```

---

## 3. C# / .NET / Unity / Godot Integration

Use [`bindings/csharp/Duct.cs`](file:///c:/Users/reese/Desktop/duct/bindings/csharp/Duct.cs) with P/Invoke:

```csharp
using Project32.Duct;

var windowHandle = new DuctWindowHandle(myHwnd);
var cfg = new DuctInitConfig {
    Window = windowHandle,
    Width = 1280,
    Height = 720,
    VSync = true,
    DebugMode = true,
    AppName = "C# Game"
};

IntPtr ductHandle;
DuctResult res = DuctNative.duct_create(ref cfg, out ductHandle);
if (res == DuctResult.Success) {
    DuctNative.duct_begin_frame(ductHandle, DuctColor.CornflowerBlue);
    DuctNative.duct_draw_instanced(ductHandle, 3, 100, 0);
    DuctNative.duct_end_frame(ductHandle);
    DuctNative.duct_present(ductHandle);

    DuctFrameStats stats;
    DuctNative.duct_get_frame_stats(ductHandle, out stats);
    Console.WriteLine($"FPS: {stats.SmoothedFps}, DrawCalls: {stats.DrawCalls}");

    DuctNative.duct_switch_backend(ductHandle, DuctBackendType.Vulkan);
    DuctNative.duct_destroy(ductHandle);
}
```

---

## 4. Window Attachment Requirement

To prevent undefined behavior and GPU crashes:
- `duct` **strictly requires** a native window to be attached (`window.native_window != NULL`).
- If no window is attached during initialization, `duct_create()` returns `DUCT_ERROR_WINDOW_REQUIRED` (`-9`).
