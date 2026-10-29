//! DirectX 11 hardware backend pipeline for Project32 duct graphics subsystem.
//!
//! Pure zero-dependency Rust implementation dynamically loading `d3d11.dll`, `dxgi.dll`,
//! and `d3dcompiler_47.dll` at runtime on Windows (with cross-platform fallback on Unix).
//! Compiles into `duct.dx11.dll` (or platform equivalent).

#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    clippy::upper_case_acronyms
)]

use duct_core::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};

// ── Raw DirectX 11 / DXGI / D3DCompiler C-ABI Definitions ───────────────────

pub type HRESULT = i32;
pub const S_OK: HRESULT = 0;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GUID {
    pub Data1: u32,
    pub Data2: u16,
    pub Data3: u16,
    pub Data4: [u8; 8],
}

// IID_ID3D11Texture2D: {6f15aaf2-d208-4e89-9ab4-489535d34f9c}
pub const IID_ID3D11Texture2D: GUID = GUID {
    Data1: 0x6f15_aaf2,
    Data2: 0xd208,
    Data3: 0x4e89,
    Data4: [0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3, 0x4f, 0x9c],
};

// IID_IDXGIDevice: {54ec77fa-1377-44e6-8c32-88fd5f44c84c}
pub const IID_IDXGIDevice: GUID = GUID {
    Data1: 0x54ec_77fa,
    Data2: 0x1377,
    Data3: 0x44e6,
    Data4: [0x8c, 0x32, 0x88, 0xfd, 0x5f, 0x44, 0xc8, 0x4c],
};

pub const D3D_DRIVER_TYPE_HARDWARE: u32 = 1;
pub const D3D_DRIVER_TYPE_WARP: u32 = 5;
pub const D3D11_SDK_VERSION: u32 = 7;

pub const D3D_FEATURE_LEVEL_11_1: u32 = 0xb100;
pub const D3D_FEATURE_LEVEL_11_0: u32 = 0xb000;
pub const D3D_FEATURE_LEVEL_10_1: u32 = 0xa100;
pub const D3D_FEATURE_LEVEL_10_0: u32 = 0xa000;

pub const DXGI_FORMAT_R32G32B32A32_FLOAT: u32 = 2;
pub const DXGI_FORMAT_R32G32B32_FLOAT: u32 = 6;
pub const DXGI_FORMAT_R16G16B16A16_FLOAT: u32 = 10;
pub const DXGI_FORMAT_R32G32_FLOAT: u32 = 16;
pub const DXGI_FORMAT_R8G8B8A8_UNORM: u32 = 28;
pub const DXGI_FORMAT_R32_FLOAT: u32 = 41;
pub const DXGI_FORMAT_R32_UINT: u32 = 42;
pub const DXGI_FORMAT_R16_UINT: u32 = 57;
pub const DXGI_FORMAT_R8_UNORM: u32 = 61;

pub const DXGI_USAGE_RENDER_TARGET_OUTPUT: u32 = 0x0000_0020;
pub const DXGI_SWAP_EFFECT_DISCARD: u32 = 0;

pub const D3D11_USAGE_DEFAULT: u32 = 0;
pub const D3D11_USAGE_STAGING: u32 = 3;

pub const D3D11_BIND_VERTEX_BUFFER: u32 = 0x1;
pub const D3D11_BIND_INDEX_BUFFER: u32 = 0x2;
pub const D3D11_BIND_CONSTANT_BUFFER: u32 = 0x4;
pub const D3D11_BIND_SHADER_RESOURCE: u32 = 0x8;
pub const D3D11_BIND_RENDER_TARGET: u32 = 0x20;

pub const D3D11_CPU_ACCESS_READ: u32 = 0x20000;
pub const D3D11_MAP_READ: u32 = 1;

pub const D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST: u32 = 4;

pub const D3D11_FILL_WIREFRAME: u32 = 2;
pub const D3D11_FILL_SOLID: u32 = 3;

pub const D3D11_CULL_NONE: u32 = 1;
pub const D3D11_CULL_FRONT: u32 = 2;
pub const D3D11_CULL_BACK: u32 = 3;

pub const D3D11_BLEND_ZERO: u32 = 1;
pub const D3D11_BLEND_ONE: u32 = 2;
pub const D3D11_BLEND_SRC_ALPHA: u32 = 5;
pub const D3D11_BLEND_INV_SRC_ALPHA: u32 = 6;
pub const D3D11_BLEND_OP_ADD: u32 = 1;
pub const D3D11_COLOR_WRITE_ENABLE_ALL: u8 = 0x0F;

pub const D3D11_DEPTH_WRITE_MASK_ZERO: u32 = 0;
pub const D3D11_DEPTH_WRITE_MASK_ALL: u32 = 1;

pub const D3D11_COMPARISON_NEVER: u32 = 1;
pub const D3D11_COMPARISON_LESS: u32 = 2;
pub const D3D11_COMPARISON_EQUAL: u32 = 3;
pub const D3D11_COMPARISON_LESS_EQUAL: u32 = 4;
pub const D3D11_COMPARISON_GREATER: u32 = 5;
pub const D3D11_COMPARISON_NOT_EQUAL: u32 = 6;
pub const D3D11_COMPARISON_GREATER_EQUAL: u32 = 7;
pub const D3D11_COMPARISON_ALWAYS: u32 = 8;

pub const D3D11_STENCIL_OP_KEEP: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D11_DEPTH_STENCILOP_DESC {
    pub StencilFailOp: u32,
    pub StencilDepthFailOp: u32,
    pub StencilPassOp: u32,
    pub StencilFunc: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D11_DEPTH_STENCIL_DESC {
    pub DepthEnable: i32,
    pub DepthWriteMask: u32,
    pub DepthFunc: u32,
    pub StencilEnable: i32,
    pub StencilReadMask: u8,
    pub StencilWriteMask: u8,
    pub FrontFace: D3D11_DEPTH_STENCILOP_DESC,
    pub BackFace: D3D11_DEPTH_STENCILOP_DESC,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Dx11RasterizerKey {
    pub(crate) cull_mode: DuctCullMode,
    pub(crate) wireframe: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Dx11BlendKey {
    pub(crate) blend_mode: DuctBlendMode,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Dx11DepthStencilKey {
    pub(crate) depth_test: bool,
    pub(crate) depth_write: bool,
    pub(crate) depth_compare: DuctCompareOp,
}

pub const D3D11_FILTER_MIN_MAG_MIP_POINT: u32 = 0;
pub const D3D11_FILTER_MIN_MAG_MIP_LINEAR: u32 = 0x15;
pub const D3D11_TEXTURE_ADDRESS_WRAP: u32 = 1;
pub const D3D11_TEXTURE_ADDRESS_MIRROR: u32 = 2;
pub const D3D11_TEXTURE_ADDRESS_CLAMP: u32 = 3;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct DXGI_RATIONAL {
    pub Numerator: u32,
    pub Denominator: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct DXGI_MODE_DESC {
    pub Width: u32,
    pub Height: u32,
    pub RefreshRate: DXGI_RATIONAL,
    pub Format: u32,
    pub ScanlineOrdering: u32,
    pub Scaling: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DXGI_SAMPLE_DESC {
    pub Count: u32,
    pub Quality: u32,
}

impl Default for DXGI_SAMPLE_DESC {
    fn default() -> Self {
        Self {
            Count: 1,
            Quality: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DXGI_SWAP_CHAIN_DESC {
    pub BufferDesc: DXGI_MODE_DESC,
    pub SampleDesc: DXGI_SAMPLE_DESC,
    pub BufferUsage: u32,
    pub BufferCount: u32,
    pub OutputWindow: *mut c_void,
    pub Windowed: i32,
    pub SwapEffect: u32,
    pub Flags: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DXGI_ADAPTER_DESC {
    pub Description: [u16; 128],
    pub VendorId: u32,
    pub DeviceId: u32,
    pub SubSysId: u32,
    pub Revision: u32,
    pub DedicatedVideoMemory: usize,
    pub DedicatedSystemMemory: usize,
    pub SharedSystemMemory: usize,
    pub AdapterLuidLow: u32,
    pub AdapterLuidHigh: i32,
}

impl Default for DXGI_ADAPTER_DESC {
    fn default() -> Self {
        Self {
            Description: [0; 128],
            VendorId: 0,
            DeviceId: 0,
            SubSysId: 0,
            Revision: 0,
            DedicatedVideoMemory: 0,
            DedicatedSystemMemory: 0,
            SharedSystemMemory: 0,
            AdapterLuidLow: 0,
            AdapterLuidHigh: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D11_BUFFER_DESC {
    pub ByteWidth: u32,
    pub Usage: u32,
    pub BindFlags: u32,
    pub CPUAccessFlags: u32,
    pub MiscFlags: u32,
    pub StructureByteStride: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_SUBRESOURCE_DATA {
    pub pSysMem: *const c_void,
    pub SysMemPitch: u32,
    pub SysMemSlicePitch: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D11_TEXTURE2D_DESC {
    pub Width: u32,
    pub Height: u32,
    pub MipLevels: u32,
    pub ArraySize: u32,
    pub Format: u32,
    pub SampleDesc: DXGI_SAMPLE_DESC,
    pub Usage: u32,
    pub BindFlags: u32,
    pub CPUAccessFlags: u32,
    pub MiscFlags: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_INPUT_ELEMENT_DESC {
    pub SemanticName: *const c_char,
    pub SemanticIndex: u32,
    pub Format: u32,
    pub InputSlot: u32,
    pub AlignedByteOffset: u32,
    pub InputSlotClass: u32,
    pub InstanceDataStepRate: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_VIEWPORT {
    pub TopLeftX: f32,
    pub TopLeftY: f32,
    pub Width: f32,
    pub Height: f32,
    pub MinDepth: f32,
    pub MaxDepth: f32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_BOX {
    pub left: u32,
    pub top: u32,
    pub front: u32,
    pub right: u32,
    pub bottom: u32,
    pub back: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_MAPPED_SUBRESOURCE {
    pub pData: *mut c_void,
    pub RowPitch: u32,
    pub DepthPitch: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_RASTERIZER_DESC {
    pub FillMode: u32,
    pub CullMode: u32,
    pub FrontCounterClockwise: i32,
    pub DepthBias: i32,
    pub DepthBiasClamp: f32,
    pub SlopeScaledDepthBias: f32,
    pub DepthClipEnable: i32,
    pub ScissorEnable: i32,
    pub MultisampleEnable: i32,
    pub AntialiasedLineEnable: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D11_RENDER_TARGET_BLEND_DESC {
    pub BlendEnable: i32,
    pub SrcBlend: u32,
    pub DestBlend: u32,
    pub BlendOp: u32,
    pub SrcBlendAlpha: u32,
    pub DestBlendAlpha: u32,
    pub BlendOpAlpha: u32,
    pub RenderTargetWriteMask: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_BLEND_DESC {
    pub AlphaToCoverageEnable: i32,
    pub IndependentBlendEnable: i32,
    pub RenderTarget: [D3D11_RENDER_TARGET_BLEND_DESC; 8],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D11_SAMPLER_DESC {
    pub Filter: u32,
    pub AddressU: u32,
    pub AddressV: u32,
    pub AddressW: u32,
    pub MipLODBias: f32,
    pub MaxAnisotropy: u32,
    pub ComparisonFunc: u32,
    pub BorderColor: [f32; 4],
    pub MinLOD: f32,
    pub MaxLOD: f32,
}

// ── COM VTable Helpers ──────────────────────────────────────────────────────

#[inline]
pub(crate) unsafe fn com_vtable(obj: *mut c_void) -> *const *const c_void {
    *(obj as *const *const *const c_void)
}

#[inline]
pub(crate) unsafe fn com_release(obj: *mut c_void) {
    if !obj.is_null() {
        let vt = com_vtable(obj);
        let release: unsafe extern "system" fn(*mut c_void) -> u32 =
            std::mem::transmute(*vt.add(2));
        release(obj);
    }
}

#[inline]
pub(crate) unsafe fn com_query_interface(
    obj: *mut c_void,
    riid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    let vt = com_vtable(obj);
    let qi: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT =
        std::mem::transmute(*vt.add(0));
    qi(obj, riid, out)
}

#[inline]
pub(crate) unsafe fn blob_pointer(blob: *mut c_void) -> *const u8 {
    let vt = com_vtable(blob);
    let f: unsafe extern "system" fn(*mut c_void) -> *const u8 = std::mem::transmute(*vt.add(3));
    f(blob)
}

#[inline]
pub(crate) unsafe fn blob_size(blob: *mut c_void) -> usize {
    let vt = com_vtable(blob);
    let f: unsafe extern "system" fn(*mut c_void) -> usize = std::mem::transmute(*vt.add(4));
    f(blob)
}

// ── Dynamic Library Loader for D3D11 & D3DCompiler ──────────────────────────

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryA(lpLibFileName: *const c_char) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *mut c_void;
    fn FreeLibrary(hLibModule: *mut c_void) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    pub(crate) fn IsWindow(hWnd: *mut c_void) -> i32;
}

pub type PFN_D3D11CreateDeviceAndSwapChain = unsafe extern "system" fn(
    pAdapter: *mut c_void,
    DriverType: u32,
    Software: *mut c_void,
    Flags: u32,
    pFeatureLevels: *const u32,
    FeatureLevels: u32,
    SDKVersion: u32,
    pSwapChainDesc: *const DXGI_SWAP_CHAIN_DESC,
    ppSwapChain: *mut *mut c_void,
    ppDevice: *mut *mut c_void,
    pFeatureLevel: *mut u32,
    ppImmediateContext: *mut *mut c_void,
) -> HRESULT;

pub type PFN_D3D11CreateDevice = unsafe extern "system" fn(
    pAdapter: *mut c_void,
    DriverType: u32,
    Software: *mut c_void,
    Flags: u32,
    pFeatureLevels: *const u32,
    FeatureLevels: u32,
    SDKVersion: u32,
    ppDevice: *mut *mut c_void,
    pFeatureLevel: *mut u32,
    ppImmediateContext: *mut *mut c_void,
) -> HRESULT;

pub type PFN_D3DCompile = unsafe extern "system" fn(
    pSrcData: *const c_void,
    SrcDataSize: usize,
    pSourceName: *const c_char,
    pDefines: *const c_void,
    pInclude: *const c_void,
    pEntrypoint: *const c_char,
    pTarget: *const c_char,
    Flags1: u32,
    Flags2: u32,
    ppCode: *mut *mut c_void,
    ppErrorMsgs: *mut *mut c_void,
) -> HRESULT;

pub(crate) struct D3D11Loader {
    #[cfg(target_os = "windows")]
    pub(crate) d3d11_lib: *mut c_void,
    #[cfg(target_os = "windows")]
    pub(crate) d3dcompiler_lib: *mut c_void,
    pub(crate) create_device_and_swapchain: Option<PFN_D3D11CreateDeviceAndSwapChain>,
    pub(crate) create_device: Option<PFN_D3D11CreateDevice>,
    pub(crate) d3d_compile: Option<PFN_D3DCompile>,
}

impl D3D11Loader {
    pub(crate) unsafe fn new() -> Option<Self> {
        #[cfg(target_os = "windows")]
        {
            let d3d11_lib = LoadLibraryA(b"d3d11.dll\0".as_ptr() as *const c_char);
            if d3d11_lib.is_null() {
                return None;
            }
            let d3dcompiler_lib = {
                let mut h = LoadLibraryA(b"d3dcompiler_47.dll\0".as_ptr() as *const c_char);
                if h.is_null() {
                    h = LoadLibraryA(b"d3dcompiler_43.dll\0".as_ptr() as *const c_char);
                }
                h
            };

            let create_device_and_swapchain = {
                let p = GetProcAddress(
                    d3d11_lib,
                    b"D3D11CreateDeviceAndSwapChain\0".as_ptr() as *const c_char,
                );
                if p.is_null() {
                    None
                } else {
                    Some(std::mem::transmute::<
                        *mut c_void,
                        PFN_D3D11CreateDeviceAndSwapChain,
                    >(p))
                }
            };

            let create_device = {
                let p = GetProcAddress(d3d11_lib, b"D3D11CreateDevice\0".as_ptr() as *const c_char);
                if p.is_null() {
                    None
                } else {
                    Some(std::mem::transmute::<*mut c_void, PFN_D3D11CreateDevice>(p))
                }
            };

            let d3d_compile = if !d3dcompiler_lib.is_null() {
                let p = GetProcAddress(d3dcompiler_lib, b"D3DCompile\0".as_ptr() as *const c_char);
                if p.is_null() {
                    None
                } else {
                    Some(std::mem::transmute::<*mut c_void, PFN_D3DCompile>(p))
                }
            } else {
                None
            };

            Some(Self {
                d3d11_lib,
                d3dcompiler_lib,
                create_device_and_swapchain,
                create_device,
                d3d_compile,
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            None
        }
    }
}

impl Drop for D3D11Loader {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            if !self.d3dcompiler_lib.is_null() {
                FreeLibrary(self.d3dcompiler_lib);
            }
            if !self.d3d11_lib.is_null() {
                FreeLibrary(self.d3d11_lib);
            }
        }
    }
}

// ── Built-in HLSL Shaders (Used when GLSL or default shaders are bound) ─────

pub(crate) const BUILTIN_VS_HLSL: &[u8] = b"
cbuffer PushConstants : register(b0) {
    float4 uColor;
    float4 uParams;
    row_major float4x4 uTransform;
};
struct VSInput {
    float3 pos : POSITION;
};
struct PSInput {
    float4 pos : SV_POSITION;
    float4 color : COLOR0;
};
PSInput main(VSInput input) {
    PSInput o;
    float2 scaled = input.pos.xy * uParams.zw + uParams.xy;
    o.pos = mul(float4(scaled, input.pos.z, 1.0), uTransform);
    o.color = float4(1.0, 1.0, 1.0, 1.0);
    return o;
}
";

pub(crate) const BUILTIN_VS_COLOR_HLSL: &[u8] = b"
cbuffer PushConstants : register(b0) {
    float4 uColor;
    float4 uParams;
    row_major float4x4 uTransform;
};
struct VSInput {
    float3 pos : POSITION;
    float4 color : COLOR0;
};
struct PSInput {
    float4 pos : SV_POSITION;
    float4 color : COLOR0;
};
PSInput main(VSInput input) {
    PSInput o;
    float2 scaled = input.pos.xy * uParams.zw + uParams.xy;
    o.pos = mul(float4(scaled, input.pos.z, 1.0), uTransform);
    o.color = input.color;
    return o;
}
";

pub(crate) const BUILTIN_PS_HLSL: &[u8] = b"
cbuffer PushConstants : register(b0) {
    float4 uColor;
    float4 uParams;
    row_major float4x4 uTransform;
};
struct PSInput {
    float4 pos : SV_POSITION;
    float4 color : COLOR0;
};
float4 main(PSInput input) : SV_TARGET {
    return input.color * uColor;
}
";

const BUILTIN_CS_HLSL: &[u8] = b"
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID) {}
";

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct Dx11ConstantBlock {
    pub(crate) color: [f32; 4],
    pub(crate) params: [f32; 4],
    pub(crate) transform: [f32; 16],
}

impl Default for Dx11ConstantBlock {
    fn default() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            params: [0.0, 0.0, 1.0, 1.0],
            transform: [
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

pub(crate) struct Dx11ShaderRecord {
    pub(crate) stage: DuctShaderStage,
    pub(crate) shader_obj: *mut c_void,
    pub(crate) shader_obj_color: *mut c_void,
    pub(crate) bytecode: Vec<u8>,
    pub(crate) bytecode_color: Vec<u8>,
}

pub(crate) struct Dx11BufferRecord {
    pub(crate) buffer: *mut c_void,
    pub(crate) size_bytes: usize,
    pub(crate) buffer_type: DuctBufferType,
}

pub(crate) struct Dx11TextureRecord {
    pub(crate) texture: *mut c_void,
    pub(crate) srv: *mut c_void,
    pub(crate) sampler: *mut c_void,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: u32,
    pub(crate) min_filter: DuctFilterMode,
    pub(crate) mag_filter: DuctFilterMode,
    pub(crate) wrap_s: DuctWrapMode,
    pub(crate) wrap_t: DuctWrapMode,
}

pub(crate) struct Dx11RenderTargetRecord {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) color_tex: DuctTextureHandle,
    pub(crate) rtv: *mut c_void,
}

pub struct Dx11Pipeline {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) extra_info: CString,
    pub(crate) initialized: bool,
    pub(crate) viewport_width: u32,
    pub(crate) viewport_height: u32,
    pub(crate) vsync_enabled: bool,
    pub(crate) wireframe: bool,

    pub(crate) loader: Option<D3D11Loader>,
    pub(crate) device: *mut c_void,
    pub(crate) context: *mut c_void,
    pub(crate) swapchain: *mut c_void,
    pub(crate) backbuffer_tex: *mut c_void,
    pub(crate) backbuffer_rtv: *mut c_void,

    pub(crate) default_vs: *mut c_void,
    pub(crate) default_vs_color: *mut c_void,
    pub(crate) default_ps: *mut c_void,
    pub(crate) default_input_layout: *mut c_void,
    pub(crate) default_color_input_layout: *mut c_void,
    pub(crate) constant_buffer: *mut c_void,
    pub(crate) constants: Dx11ConstantBlock,

    pub(crate) current_pso: Option<DuctPipelineStateDesc>,
    pub(crate) active_render_target: DuctRenderTargetHandle,
    pub(crate) last_bound_vbo: *mut c_void,
    pub(crate) last_bound_ibo: *mut c_void,
    pub(crate) current_index_format: DuctIndexFormat,

    pub(crate) frame_in_progress: bool,
    pub(crate) frame_count: u64,
    pub(crate) draw_calls_in_frame: u32,
    pub(crate) vertices_drawn_in_frame: u32,
    pub(crate) pipeline_switches_in_frame: u32,
    pub(crate) allocated_gpu_bytes: usize,
    pub(crate) last_frame_instant: Option<std::time::Instant>,
    pub(crate) smoothed_fps: f32,
    pub(crate) current_frame_time_ms: f32,

    pub(crate) next_handle_id: u64,
    pub(crate) shaders: HashMap<DuctShaderHandle, Dx11ShaderRecord>,
    pub(crate) buffers: HashMap<DuctBufferHandle, Dx11BufferRecord>,
    pub(crate) textures: HashMap<DuctTextureHandle, Dx11TextureRecord>,
    pub(crate) render_targets: HashMap<DuctRenderTargetHandle, Dx11RenderTargetRecord>,

    pub(crate) cached_rasterizer_states: HashMap<Dx11RasterizerKey, *mut c_void>,
    pub(crate) cached_blend_states: HashMap<Dx11BlendKey, *mut c_void>,
    pub(crate) cached_depth_stencil_states: HashMap<Dx11DepthStencilKey, *mut c_void>,
}

impl Default for Dx11Pipeline {
    fn default() -> Self {
        Self {
            name: "DirectX 11 (D3D11 Feature Level 11_0)".to_string(),
            version: "11.0".to_string(),
            extra_info: CString::new("DirectX 11 Hardware Pipeline").unwrap(),
            initialized: false,
            viewport_width: 1280,
            viewport_height: 720,
            vsync_enabled: true,
            wireframe: false,

            loader: None,
            device: std::ptr::null_mut(),
            context: std::ptr::null_mut(),
            swapchain: std::ptr::null_mut(),
            backbuffer_tex: std::ptr::null_mut(),
            backbuffer_rtv: std::ptr::null_mut(),

            default_vs: std::ptr::null_mut(),
            default_vs_color: std::ptr::null_mut(),
            default_ps: std::ptr::null_mut(),
            default_input_layout: std::ptr::null_mut(),
            default_color_input_layout: std::ptr::null_mut(),
            constant_buffer: std::ptr::null_mut(),
            constants: Dx11ConstantBlock::default(),

            current_pso: None,
            active_render_target: DuctRenderTargetHandle::INVALID,
            last_bound_vbo: std::ptr::null_mut(),
            last_bound_ibo: std::ptr::null_mut(),
            current_index_format: DuctIndexFormat::U16,

            frame_in_progress: false,
            frame_count: 0,
            draw_calls_in_frame: 0,
            vertices_drawn_in_frame: 0,
            pipeline_switches_in_frame: 0,
            allocated_gpu_bytes: 0,
            last_frame_instant: None,
            smoothed_fps: 60.0,
            current_frame_time_ms: 16.67,

            next_handle_id: 1,
            shaders: HashMap::new(),
            buffers: HashMap::new(),
            textures: HashMap::new(),
            render_targets: HashMap::new(),
            cached_rasterizer_states: HashMap::new(),
            cached_blend_states: HashMap::new(),
            cached_depth_stencil_states: HashMap::new(),
        }
    }
}

pub(crate) unsafe extern "C" fn dx11_get_proc_address(_name: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}

