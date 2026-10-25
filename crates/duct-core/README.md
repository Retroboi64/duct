# duct-core

`duct-core` defines the zero-dependency C-ABI vtable interface, primitive types, handles, and enumerations for the **Duct** modular graphics subsystem in the Project32 engine ecosystem.

## Features
- Pure standard library (`no_std` compatible structs and C-ABI layout).
- C-compatible primitive types: `DuctColor`, `DuctWindowHandle`, `DuctInitConfig`, `DuctShaderDesc`, `DuctBufferDesc`, `DuctRenderTargetDesc`, `DuctPipelineStateDesc`, `DuctFrameStats`, `DuctRawContext`.
- C-ABI vtable interface: `DuctPipelineVTable` defining backend function pointers.
- Unified backend and error enumerations: `DuctResult`, `DuctBackendType`, `DuctShaderStage`, `DuctBufferType`, `DuctTextureFormat`, `DuctFilterMode`, `DuctWrapMode`, `DuctCompareOp`, `DuctCullMode`, `DuctBlendMode`, `DuctIndexFormat`, `DuctVertexFormat`.

## License
Dual-licensed under MIT or Apache-2.0.
