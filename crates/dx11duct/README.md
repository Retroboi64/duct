# dx11duct

Hardware-accelerated DirectX 11 graphics backend pipeline module for the [Duct](https://crates.io/crates/duct) rendering subsystem.

## Features
- Pure standard library implementation with zero external dependencies.
- Hardware `D3D11CreateDeviceAndSwapChain` and DXGI swapchain presentation.
- Dynamic HLSL shader compilation via `d3dcompiler_47.dll` (`vs_5_0`, `ps_5_0`, `cs_5_0`).
- Vertex/Index/Constant buffers, 2D textures, and offscreen RenderTargetViews.
- Staging texture pixel readback and live shader hot-reloading.

## License
Dual-licensed under MIT or Apache-2.0.
