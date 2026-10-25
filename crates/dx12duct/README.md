# dx12duct

Hardware-accelerated DirectX 12 graphics backend pipeline module for the [Duct](https://crates.io/crates/duct) rendering subsystem.

## Features
- Pure standard library implementation with zero external dependencies.
- Hardware `D3D12CreateDevice` (Feature Level 12_1) and `ID3D12CommandQueue` creation.
- DXGI swapchain presentation, dynamic HLSL compilation, and buffer/texture management.
- Offscreen render targets, staging readbacks, and live shader hot-reloading.

## License
Dual-licensed under MIT or Apache-2.0.
