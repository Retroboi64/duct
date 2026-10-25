# glduct

Hardware-accelerated OpenGL 3.3 Core graphics backend pipeline module for the [Duct](https://crates.io/crates/duct) rendering subsystem.

## Features
- Pure standard library implementation with zero external dependencies.
- Hardware WGL (Windows), GLX (Linux), and CGL (macOS) context creation.
- Vertex/Index/Uniform buffers, 2D textures, mipmap generation, texture filtering/wrapping.
- Offscreen Framebuffer Objects (FBOs), blitting, and pixel readback.
- Dynamic shader compilation, live reloading, and compute shader dispatch.

## License
Dual-licensed under MIT or Apache-2.0.
