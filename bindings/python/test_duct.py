import sys
import os
import struct

# Add bindings/python to path
sys.path.insert(0, os.path.dirname(__file__))

from duct import Duct

print("[Python Test] Testing Project32 Duct Python FFI...")

# 1. Test window required check
try:
    Duct(hwnd=0)
    print("[Python Test] ERROR: Duct allowed null HWND!")
    sys.exit(1)
except RuntimeError as e:
    print(f"[Python Test] Window requirement verified: {e}")

# 2. Test initialization with dummy native window handle (0x12345)
# Using DirectX 12 stub backend for testing on mock window handle
sys.argv.append("--dx12")
duct = Duct.from_args(hwnd=0x12345)
print(f"[Python Test] Loaded Backend: {duct.backend_name}")

# 3. Test frame operations
duct.resize_viewport(1920, 1080)
duct.begin_frame(0.39, 0.58, 0.93, 1.0)

# 4. Test shader creation & hot-reloading
vs_source = """#version 330 core
layout(location=0) in vec3 pos;
void main() { gl_Position = vec4(pos, 1.0); }"""

fs_source = """#version 330 core
out vec4 color;
void main() { color = vec4(1.0, 0.0, 0.0, 1.0); }"""

vs = duct.create_shader(1, vs_source)
fs = duct.create_shader(2, fs_source)
print(f"[Python Test] Created Shaders: VS={vs}, FS={fs}")

duct.reload_shader(fs, 2, fs_source)
print("[Python Test] Shader hot-reloading verified.")

# 5. Test buffer creation (triangle)
triangle = struct.pack("9f", 0.0, 0.5, 0.0, -0.5, -0.5, 0.0, 0.5, -0.5, 0.0)
vbo = duct.create_buffer(1, triangle)
print(f"[Python Test] Created VBO Buffer: {vbo}")

# 6. Test Offscreen Render Target (FBO)
rt = duct.create_render_target(800, 600, format=1, has_depth=True)
print(f"[Python Test] Created Offscreen Render Target FBO: {rt}")
duct.bind_render_target(rt)
duct.bind_render_target(0) # Restore default backbuffer

# 7. Test PSO, Draw, and Instancing
duct.set_pipeline_state(vs, fs)
duct.draw(3, 0)
duct.draw_instanced(3, 10)
duct.end_frame()
duct.present()

# 8. Test Telemetry & Frame Stats
stats = duct.get_frame_stats()
print(f"[Python Test] Telemetry: FPS={stats['fps']:.1f}, DrawCalls={stats['draw_calls']}, Triangles={stats['triangles_rendered']}")

# 9. Test Live Backend Switching
print("[Python Test] Testing Live Backend Switching to DirectX 11...")
duct.switch_backend(5) # DirectX 11
print(f"[Python Test] Switched Backend: {duct.backend_name}")

# 10. Test Base Shape Drawing on DirectX 11
duct.begin_frame(0.1, 0.1, 0.2, 1.0)
duct.draw_triangle(0.0, 0.5, -0.5, -0.5, 0.5, -0.5, 1.0, 0.5, 0.2, 1.0)
duct.draw_rect(-0.8, 0.4, 0.3, 0.3, 0.2, 0.85, 0.4, 1.0)
duct.draw_circle(0.6, 0.5, 0.2, 32, 0.2, 0.7, 1.0, 1.0)
duct.end_frame()
duct.present()

print("[Python Test] Testing Live Backend Switching to DirectX 12...")
duct.switch_backend(2) # DirectX 12
print(f"[Python Test] Switched Backend: {duct.backend_name}")

# 11. Test Base Shape Drawing on DirectX 12
duct.begin_frame(0.1, 0.1, 0.2, 1.0)
duct.draw_triangle(0.0, 0.5, -0.5, -0.5, 0.5, -0.5, 1.0, 0.5, 0.2, 1.0)
duct.draw_rect(-0.8, 0.4, 0.3, 0.3, 0.2, 0.85, 0.4, 1.0)
duct.draw_circle(0.6, 0.5, 0.2, 32, 0.2, 0.7, 1.0, 1.0)
duct.end_frame()
duct.present()

print("[Python Test] Testing Live Backend Switching to Vulkan...")
duct.switch_backend(3) # Vulkan
print(f"[Python Test] Switched Backend: {duct.backend_name}")

# 12. Test Base Shape Drawing on Vulkan
duct.begin_frame(0.1, 0.1, 0.2, 1.0)
duct.draw_triangle(0.0, 0.5, -0.5, -0.5, 0.5, -0.5, 1.0, 0.5, 0.2, 1.0)
duct.draw_rect(-0.8, 0.4, 0.3, 0.3, 0.2, 0.85, 0.4, 1.0)
duct.draw_circle(0.6, 0.5, 0.2, 32, 0.2, 0.7, 1.0, 1.0)
duct.end_frame()
duct.present()

print("[Python Test] All Python FFI operations completed successfully!")
