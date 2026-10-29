# vkduct

Hardware-accelerated Vulkan 1.3 graphics backend pipeline module for the [Duct](https://crates.io/crates/duct) rendering subsystem.

## Features
- Pure standard library implementation with zero external dependencies.
- Hardware `vkCreateInstance`, physical device selection, queue family selection, and swapchains (`VK_KHR_swapchain`).
- Runtime SPIR-V 1.0 shader assembler.
- Vertex/Index/Uniform buffers, 2D textures, samplers, and offscreen render passes.
- Staging buffer pixel readback and live shader hot-reloading.

## License
Licensed under MIT.
