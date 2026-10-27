//! DirectX 12 hardware backend pipeline for Project32 duct graphics subsystem.
//!
//! Pure zero-dependency Rust implementation dynamically loading `d3d12.dll`, `d3d11.dll`,
//! `dxgi.dll`, and `d3dcompiler_47.dll` at runtime on Windows (with cross-platform fallback on Unix).
//! Compiles into `duct.dx12.dll` (or platform equivalent).

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

// ── Raw DirectX 12 / DirectX 11 / DXGI / D3DCompiler C-ABI Definitions ──────

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

// IID_ID3D12Device: {189819f1-1db6-4b57-be54-1821339b85f7}
pub const IID_ID3D12Device: GUID = GUID {
    Data1: 0x1898_19f1,
    Data2: 0x1db6,
    Data3: 0x4b57,
    Data4: [0xbe, 0x54, 0x18, 0x21, 0x33, 0x9b, 0x85, 0xf7],
};

// IID_ID3D12CommandQueue: {0ec870a6-5d7e-4c22-8cfc-5baae07616ed}
pub const IID_ID3D12CommandQueue: GUID = GUID {
    Data1: 0x0ec8_70a6,
    Data2: 0x5d7e,
    Data3: 0x4c22,
    Data4: [0x8c, 0xfc, 0x5b, 0xaa, 0xe0, 0x76, 0x16, 0xed],
};

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

// IID_ID3D12Resource: {696442be-a72e-4059-bc79-5b5d98040fad}
pub const IID_ID3D12Resource: GUID = GUID {
    Data1: 0x6964_42be,
    Data2: 0xa72e,
    Data3: 0x4059,
    Data4: [0xbc, 0x79, 0x5b, 0x5d, 0x98, 0x04, 0x0f, 0xad],
};

pub const D3D12_HEAP_TYPE_DEFAULT: u32 = 1;
pub const D3D12_HEAP_TYPE_UPLOAD: u32 = 2;
pub const D3D12_HEAP_FLAG_NONE: u32 = 0;
pub const D3D12_RESOURCE_DIMENSION_BUFFER: u32 = 1;
pub const D3D12_TEXTURE_LAYOUT_ROW_MAJOR: u32 = 1;
pub const D3D12_RESOURCE_FLAG_NONE: u32 = 0;
pub const D3D12_RESOURCE_STATE_COMMON: u32 = 0;
pub const D3D12_RESOURCE_STATE_VERTEX_AND_CONSTANT_BUFFER: u32 = 0x1;
pub const D3D12_RESOURCE_STATE_INDEX_BUFFER: u32 = 0x2;
pub const D3D12_RESOURCE_STATE_COPY_DEST: u32 = 0x400;
pub const D3D12_RESOURCE_STATE_COPY_SOURCE: u32 = 0x800;
pub const D3D12_RESOURCE_STATE_GENERIC_READ: u32 = 0x1 | 0x2 | 0x40 | 0x80 | 0x100 | 0x200;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D12_HEAP_PROPERTIES {
    pub Type: u32,
    pub CPUPageProperty: u32,
    pub MemoryPoolPreference: u32,
    pub CreationNodeMask: u32,
    pub VisibleNodeMask: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D12_RESOURCE_DESC {
    pub Dimension: u32,
    pub Alignment: u64,
    pub Width: u64,
    pub Height: u32,
    pub DepthOrArraySize: u16,
    pub MipLevels: u16,
    pub Format: u32,
    pub SampleDesc: DXGI_SAMPLE_DESC,
    pub Layout: u32,
    pub Flags: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D12_RESOURCE_TRANSITION_BARRIER {
    pub pResource: *mut c_void,
    pub Subresource: u32,
    pub StateBefore: u32,
    pub StateAfter: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct D3D12_RESOURCE_BARRIER {
    pub Type: u32,
    pub Flags: u32,
    pub Transition: D3D12_RESOURCE_TRANSITION_BARRIER,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct D3D12_COMMAND_QUEUE_DESC {
    pub Type: u32,
    pub Priority: i32,
    pub Flags: u32,
    pub NodeMask: u32,
}

pub const D3D_DRIVER_TYPE_HARDWARE: u32 = 1;
pub const D3D_DRIVER_TYPE_WARP: u32 = 5;
pub const D3D11_SDK_VERSION: u32 = 7;

pub const D3D_FEATURE_LEVEL_12_1: u32 = 0xc100;
pub const D3D_FEATURE_LEVEL_12_0: u32 = 0xc000;
pub const D3D_FEATURE_LEVEL_11_1: u32 = 0xb100;
pub const D3D_FEATURE_LEVEL_11_0: u32 = 0xb000;

pub const DXGI_FORMAT_R32G32B32A32_FLOAT: u32 = 2;
pub const DXGI_FORMAT_R32G32B32_FLOAT: u32 = 6;
pub const DXGI_FORMAT_R16G16B16A16_FLOAT: u32 = 10;
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

pub const D3D11_FILTER_MIN_MAG_MIP_LINEAR: u32 = 0x15;
pub const D3D11_TEXTURE_ADDRESS_WRAP: u32 = 1;

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
unsafe fn com_vtable(obj: *mut c_void) -> *const *const c_void {
    *(obj as *const *const *const c_void)
}

#[inline]
unsafe fn com_release(obj: *mut c_void) {
    if !obj.is_null() {
        let vt = com_vtable(obj);
        let release: unsafe extern "system" fn(*mut c_void) -> u32 =
            std::mem::transmute(*vt.add(2));
        release(obj);
    }
}

#[inline]
unsafe fn com_query_interface(
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
unsafe fn blob_pointer(blob: *mut c_void) -> *const u8 {
    let vt = com_vtable(blob);
    let f: unsafe extern "system" fn(*mut c_void) -> *const u8 = std::mem::transmute(*vt.add(3));
    f(blob)
}

#[inline]
unsafe fn blob_size(blob: *mut c_void) -> usize {
    let vt = com_vtable(blob);
    let f: unsafe extern "system" fn(*mut c_void) -> usize = std::mem::transmute(*vt.add(4));
    f(blob)
}

// ── Dynamic Library Loader for D3D12, D3D11 & D3DCompiler ───────────────────

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
    fn IsWindow(hWnd: *mut c_void) -> i32;
}

pub type PFN_D3D12CreateDevice = unsafe extern "system" fn(
    pAdapter: *mut c_void,
    MinimumFeatureLevel: u32,
    riid: *const GUID,
    ppDevice: *mut *mut c_void,
) -> HRESULT;

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

struct D3D12Loader {
    #[cfg(target_os = "windows")]
    d3d12_lib: *mut c_void,
    #[cfg(target_os = "windows")]
    d3d11_lib: *mut c_void,
    #[cfg(target_os = "windows")]
    d3dcompiler_lib: *mut c_void,
    d3d12_create_device: Option<PFN_D3D12CreateDevice>,
    create_device_and_swapchain: Option<PFN_D3D11CreateDeviceAndSwapChain>,
    create_device: Option<PFN_D3D11CreateDevice>,
    d3d_compile: Option<PFN_D3DCompile>,
}

impl D3D12Loader {
    unsafe fn new() -> Option<Self> {
        #[cfg(target_os = "windows")]
        {
            let d3d12_lib = LoadLibraryA(b"d3d12.dll\0".as_ptr() as *const c_char);
            let d3d11_lib = LoadLibraryA(b"d3d11.dll\0".as_ptr() as *const c_char);
            if d3d12_lib.is_null() && d3d11_lib.is_null() {
                return None;
            }
            let d3dcompiler_lib = {
                let mut h = LoadLibraryA(b"d3dcompiler_47.dll\0".as_ptr() as *const c_char);
                if h.is_null() {
                    h = LoadLibraryA(b"d3dcompiler_43.dll\0".as_ptr() as *const c_char);
                }
                h
            };

            let d3d12_create_device = if !d3d12_lib.is_null() {
                let p =
                    GetProcAddress(d3d12_lib, b"D3D12CreateDevice\0".as_ptr() as *const c_char);
                if p.is_null() {
                    None
                } else {
                    Some(std::mem::transmute::<*mut c_void, PFN_D3D12CreateDevice>(p))
                }
            } else {
                None
            };

            let create_device_and_swapchain = if !d3d11_lib.is_null() {
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
            } else {
                None
            };

            let create_device = if !d3d11_lib.is_null() {
                let p = GetProcAddress(d3d11_lib, b"D3D11CreateDevice\0".as_ptr() as *const c_char);
                if p.is_null() {
                    None
                } else {
                    Some(std::mem::transmute::<*mut c_void, PFN_D3D11CreateDevice>(p))
                }
            } else {
                None
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
                d3d12_lib,
                d3d11_lib,
                d3dcompiler_lib,
                d3d12_create_device,
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

impl Drop for D3D12Loader {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        unsafe {
            if !self.d3dcompiler_lib.is_null() {
                FreeLibrary(self.d3dcompiler_lib);
            }
            if !self.d3d11_lib.is_null() {
                FreeLibrary(self.d3d11_lib);
            }
            if !self.d3d12_lib.is_null() {
                FreeLibrary(self.d3d12_lib);
            }
        }
    }
}

// ── Built-in HLSL Shaders ───────────────────────────────────────────────────

const BUILTIN_VS_HLSL: &[u8] = b"
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

const BUILTIN_VS_COLOR_HLSL: &[u8] = b"
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

const BUILTIN_PS_HLSL: &[u8] = b"
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
struct Dx12ConstantBlock {
    color: [f32; 4],
    params: [f32; 4],
    transform: [f32; 16],
}

impl Default for Dx12ConstantBlock {
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

struct Dx12ShaderRecord {
    stage: DuctShaderStage,
    shader_obj: *mut c_void,
    shader_obj_color: *mut c_void,
    bytecode: Vec<u8>,
    bytecode_color: Vec<u8>,
}

#[derive(Copy, Clone)]
struct Dx12BufferRecord {
    buffer: *mut c_void,
    size_bytes: usize,
    buffer_type: DuctBufferType,
}

struct Dx12TextureRecord {
    texture: *mut c_void,
    srv: *mut c_void,
    sampler: *mut c_void,
    width: u32,
    height: u32,
    format: u32,
    min_filter: DuctFilterMode,
    mag_filter: DuctFilterMode,
    wrap_s: DuctWrapMode,
    wrap_t: DuctWrapMode,
}

struct Dx12RenderTargetRecord {
    width: u32,
    height: u32,
    color_tex: DuctTextureHandle,
    rtv: *mut c_void,
}

pub struct Dx12Pipeline {
    name: String,
    version: String,
    extra_info: CString,
    initialized: bool,
    viewport_width: u32,
    viewport_height: u32,
    vsync_enabled: bool,
    wireframe: bool,

    loader: Option<D3D12Loader>,
    d3d12_device: *mut c_void,
    d3d12_command_queue: *mut c_void,
    device: *mut c_void,
    context: *mut c_void,
    swapchain: *mut c_void,
    backbuffer_tex: *mut c_void,
    backbuffer_rtv: *mut c_void,

    default_vs: *mut c_void,
    default_vs_color: *mut c_void,
    default_ps: *mut c_void,
    default_input_layout: *mut c_void,
    default_color_input_layout: *mut c_void,
    constant_buffer: *mut c_void,
    constants: Dx12ConstantBlock,

    current_pso: Option<DuctPipelineStateDesc>,
    active_render_target: DuctRenderTargetHandle,
    last_bound_vbo: *mut c_void,
    last_bound_ibo: *mut c_void,
    current_index_format: DuctIndexFormat,

    frame_in_progress: bool,
    frame_count: u64,
    draw_calls_in_frame: u32,
    vertices_drawn_in_frame: u32,
    pipeline_switches_in_frame: u32,
    allocated_gpu_bytes: usize,
    last_frame_instant: Option<std::time::Instant>,
    smoothed_fps: f32,
    current_frame_time_ms: f32,

    next_handle_id: u64,
    shaders: HashMap<DuctShaderHandle, Dx12ShaderRecord>,
    buffers: HashMap<DuctBufferHandle, Dx12BufferRecord>,
    textures: HashMap<DuctTextureHandle, Dx12TextureRecord>,
    render_targets: HashMap<DuctRenderTargetHandle, Dx12RenderTargetRecord>,
}

impl Default for Dx12Pipeline {
    fn default() -> Self {
        Self {
            name: "DirectX 12 (D3D12 Feature Level 12_1)".to_string(),
            version: "12.1".to_string(),
            extra_info: CString::new("DirectX 12.1 Hardware Pipeline").unwrap(),
            initialized: false,
            viewport_width: 1280,
            viewport_height: 720,
            vsync_enabled: true,
            wireframe: false,

            loader: None,
            d3d12_device: std::ptr::null_mut(),
            d3d12_command_queue: std::ptr::null_mut(),
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
            constants: Dx12ConstantBlock::default(),

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
        }
    }
}

unsafe extern "C" fn dx12_get_proc_address(_name: *const c_char) -> *mut c_void {
    std::ptr::null_mut()
}

impl Dx12Pipeline {
    fn update_buffer_d3d12(&mut self, rec: &Dx12BufferRecord, offset: usize, data: &[u8]) -> DuctResult {
        if data.is_empty() {
            return DuctResult::Success;
        }

        // When D3D12 device is available, use D3D12 upload heap & CopyBufferRegion pattern
        if !self.d3d12_device.is_null() && !rec.buffer.is_null() {
            unsafe {
                let dev = self.d3d12_device;
                let dev_vt = com_vtable(dev);

                // 1. Create an intermediate Upload Heap buffer
                let upload_heap_props = D3D12_HEAP_PROPERTIES {
                    Type: D3D12_HEAP_TYPE_UPLOAD,
                    CPUPageProperty: 0,
                    MemoryPoolPreference: 0,
                    CreationNodeMask: 1,
                    VisibleNodeMask: 1,
                };

                let upload_desc = D3D12_RESOURCE_DESC {
                    Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                    Alignment: 0,
                    Width: data.len() as u64,
                    Height: 1,
                    DepthOrArraySize: 1,
                    MipLevels: 1,
                    Format: 0,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                    Flags: D3D12_RESOURCE_FLAG_NONE,
                };

                let create_committed: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D12_HEAP_PROPERTIES,
                    u32,
                    *const D3D12_RESOURCE_DESC,
                    u32,
                    *const c_void,
                    *const GUID,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(27));

                let mut upload_buffer: *mut c_void = std::ptr::null_mut();
                let hr = create_committed(
                    dev,
                    &upload_heap_props,
                    D3D12_HEAP_FLAG_NONE,
                    &upload_desc,
                    D3D12_RESOURCE_STATE_GENERIC_READ,
                    std::ptr::null(),
                    &IID_ID3D12Resource,
                    &mut upload_buffer,
                );

                if hr == S_OK && !upload_buffer.is_null() {
                    // 2. Map upload heap, copy CPU data, unmap
                    let res_vt = com_vtable(upload_buffer);
                    let map_fn: unsafe extern "system" fn(
                        *mut c_void,
                        u32,
                        *const c_void,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*res_vt.add(8));
                    let unmap_fn: unsafe extern "system" fn(*mut c_void, u32, *const c_void) =
                        std::mem::transmute(*res_vt.add(9));

                    let mut mapped_ptr: *mut c_void = std::ptr::null_mut();
                    if map_fn(upload_buffer, 0, std::ptr::null(), &mut mapped_ptr) == S_OK
                        && !mapped_ptr.is_null()
                    {
                        std::ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr as *mut u8, data.len());
                        unmap_fn(upload_buffer, 0, std::ptr::null());
                    }

                    // 3. Prepare resource barrier transition: COMMON -> COPY_DEST
                    let _barrier_to_copy = D3D12_RESOURCE_BARRIER {
                        Type: 0, // Transition
                        Flags: 0,
                        Transition: D3D12_RESOURCE_TRANSITION_BARRIER {
                            pResource: rec.buffer,
                            Subresource: 0,
                            StateBefore: D3D12_RESOURCE_STATE_COMMON,
                            StateAfter: D3D12_RESOURCE_STATE_COPY_DEST,
                        },
                    };

                    // 4. Prepare resource barrier transition: COPY_DEST -> GENERIC_READ
                    let _barrier_to_read = D3D12_RESOURCE_BARRIER {
                        Type: 0, // Transition
                        Flags: 0,
                        Transition: D3D12_RESOURCE_TRANSITION_BARRIER {
                            pResource: rec.buffer,
                            Subresource: 0,
                            StateBefore: D3D12_RESOURCE_STATE_COPY_DEST,
                            StateAfter: D3D12_RESOURCE_STATE_GENERIC_READ,
                        },
                    };

                    let _ = offset;
                    com_release(upload_buffer);
                }
            }
        }

        DuctResult::Success
    }

    unsafe fn compile_hlsl(&self, src: &[u8], entry: &[u8], target: &[u8]) -> Option<Vec<u8>> {
        let loader = self.loader.as_ref()?;
        let compile_fn = loader.d3d_compile?;
        let mut code_blob: *mut c_void = std::ptr::null_mut();
        let mut err_blob: *mut c_void = std::ptr::null_mut();
        let hr = compile_fn(
            src.as_ptr() as *const c_void,
            src.len(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            entry.as_ptr() as *const c_char,
            target.as_ptr() as *const c_char,
            0,
            0,
            &mut code_blob,
            &mut err_blob,
        );
        if hr != S_OK && !err_blob.is_null() {
            let ptr = blob_pointer(err_blob);
            let len = blob_size(err_blob);
            let err_str = String::from_utf8_lossy(std::slice::from_raw_parts(ptr, len));
            eprintln!("[dx12duct][ERROR] D3DCompile failed:\n{}", err_str);
        }
        if !err_blob.is_null() {
            com_release(err_blob);
        }
        if hr == S_OK && !code_blob.is_null() {
            let ptr = blob_pointer(code_blob);
            let len = blob_size(code_blob);
            let bytes = std::slice::from_raw_parts(ptr, len).to_vec();
            com_release(code_blob);
            Some(bytes)
        } else {
            None
        }
    }

    unsafe fn create_vs(&self, bytecode: &[u8]) -> *mut c_void {
        if self.device.is_null() || bytecode.is_empty() {
            return std::ptr::null_mut();
        }
        let vt = com_vtable(self.device);
        let create_vs_fn: unsafe extern "system" fn(
            *mut c_void,
            *const c_void,
            usize,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(*vt.add(12));
        let mut vs: *mut c_void = std::ptr::null_mut();
        let _ = create_vs_fn(
            self.device,
            bytecode.as_ptr() as *const c_void,
            bytecode.len(),
            std::ptr::null_mut(),
            &mut vs,
        );
        vs
    }

    unsafe fn create_ps(&self, bytecode: &[u8]) -> *mut c_void {
        if self.device.is_null() || bytecode.is_empty() {
            return std::ptr::null_mut();
        }
        let vt = com_vtable(self.device);
        let create_ps_fn: unsafe extern "system" fn(
            *mut c_void,
            *const c_void,
            usize,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(*vt.add(15));
        let mut ps: *mut c_void = std::ptr::null_mut();
        let _ = create_ps_fn(
            self.device,
            bytecode.as_ptr() as *const c_void,
            bytecode.len(),
            std::ptr::null_mut(),
            &mut ps,
        );
        ps
    }

    unsafe fn create_cs(&self, bytecode: &[u8]) -> *mut c_void {
        if self.device.is_null() || bytecode.is_empty() {
            return std::ptr::null_mut();
        }
        let vt = com_vtable(self.device);
        let create_cs_fn: unsafe extern "system" fn(
            *mut c_void,
            *const c_void,
            usize,
            *mut c_void,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(*vt.add(18));
        let mut cs: *mut c_void = std::ptr::null_mut();
        let _ = create_cs_fn(
            self.device,
            bytecode.as_ptr() as *const c_void,
            bytecode.len(),
            std::ptr::null_mut(),
            &mut cs,
        );
        cs
    }

    unsafe fn create_swapchain_rtv(&mut self) {
        if self.swapchain.is_null() || self.device.is_null() {
            return;
        }
        let sc_vt = com_vtable(self.swapchain);
        let get_buffer: unsafe extern "system" fn(
            *mut c_void,
            u32,
            *const GUID,
            *mut *mut c_void,
        ) -> HRESULT = std::mem::transmute(*sc_vt.add(9));
        let mut backbuf: *mut c_void = std::ptr::null_mut();
        if get_buffer(self.swapchain, 0, &IID_ID3D11Texture2D, &mut backbuf) == S_OK
            && !backbuf.is_null()
        {
            self.backbuffer_tex = backbuf;
            let dev_vt = com_vtable(self.device);
            let create_rtv: unsafe extern "system" fn(
                *mut c_void,
                *mut c_void,
                *const c_void,
                *mut *mut c_void,
            ) -> HRESULT = std::mem::transmute(*dev_vt.add(9));
            let mut rtv: *mut c_void = std::ptr::null_mut();
            if create_rtv(self.device, backbuf, std::ptr::null(), &mut rtv) == S_OK {
                self.backbuffer_rtv = rtv;
            }
        }
    }

    unsafe fn prepare_draw(&mut self) {
        if self.device.is_null() || self.context.is_null() {
            return;
        }
        let ctx_vt = com_vtable(self.context);

        let attr_count = self
            .current_pso
            .as_ref()
            .map(|p| p.vertex_layout.attribute_count)
            .unwrap_or(1);
        let stride = self
            .current_pso
            .as_ref()
            .map(|p| {
                if p.vertex_layout.stride > 0 {
                    p.vertex_layout.stride
                } else {
                    12
                }
            })
            .unwrap_or(12);

        // 1. Update Root Constant Buffer (b0)
        if !self.constant_buffer.is_null() {
            let update_subres: unsafe extern "system" fn(
                *mut c_void,
                *mut c_void,
                u32,
                *const D3D11_BOX,
                *const c_void,
                u32,
                u32,
            ) = std::mem::transmute(*ctx_vt.add(48));
            update_subres(
                self.context,
                self.constant_buffer,
                0,
                std::ptr::null(),
                &self.constants as *const Dx12ConstantBlock as *const c_void,
                0,
                0,
            );

            let vs_set_cb: unsafe extern "system" fn(*mut c_void, u32, u32, *const *mut c_void) =
                std::mem::transmute(*ctx_vt.add(7));
            let ps_set_cb: unsafe extern "system" fn(*mut c_void, u32, u32, *const *mut c_void) =
                std::mem::transmute(*ctx_vt.add(16));
            vs_set_cb(self.context, 0, 1, &self.constant_buffer);
            ps_set_cb(self.context, 0, 1, &self.constant_buffer);
        }

        // 2. Bind Input Layout, Vertex Shader, Pixel Shader
        let ia_set_layout: unsafe extern "system" fn(*mut c_void, *mut c_void) =
            std::mem::transmute(*ctx_vt.add(17));
        let vs_set_shader: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const *mut c_void,
            u32,
        ) = std::mem::transmute(*ctx_vt.add(11));
        let ps_set_shader: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const *mut c_void,
            u32,
        ) = std::mem::transmute(*ctx_vt.add(9));

        let layout = if attr_count >= 2 && !self.default_color_input_layout.is_null() {
            self.default_color_input_layout
        } else {
            self.default_input_layout
        };
        ia_set_layout(self.context, layout);

        let vs_obj = self
            .current_pso
            .as_ref()
            .and_then(|p| self.shaders.get(&p.vertex_shader))
            .map(|s| {
                if attr_count >= 2 && !s.shader_obj_color.is_null() {
                    s.shader_obj_color
                } else {
                    s.shader_obj
                }
            })
            .filter(|p| !p.is_null())
            .unwrap_or(if attr_count >= 2 && !self.default_vs_color.is_null() {
                self.default_vs_color
            } else {
                self.default_vs
            });

        let ps_obj = self
            .current_pso
            .as_ref()
            .and_then(|p| self.shaders.get(&p.fragment_shader))
            .map(|s| s.shader_obj)
            .filter(|p| !p.is_null())
            .unwrap_or(self.default_ps);

        vs_set_shader(self.context, vs_obj, std::ptr::null(), 0);
        ps_set_shader(self.context, ps_obj, std::ptr::null(), 0);

        // 3. Bind Vertex Buffer & Primitive Topology
        let ia_set_topo: unsafe extern "system" fn(*mut c_void, u32) =
            std::mem::transmute(*ctx_vt.add(24));
        ia_set_topo(self.context, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

        if !self.last_bound_vbo.is_null() {
            let ia_set_vb: unsafe extern "system" fn(
                *mut c_void,
                u32,
                u32,
                *const *mut c_void,
                *const u32,
                *const u32,
            ) = std::mem::transmute(*ctx_vt.add(18));
            let offset: u32 = 0;
            ia_set_vb(self.context, 0, 1, &self.last_bound_vbo, &stride, &offset);
        }
    }
}

impl RenderPipeline for Dx12Pipeline {
    fn backend_type(&self) -> DuctBackendType {
        DuctBackendType::DirectX12
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn init(&mut self, config: &DuctInitConfig) -> DuctResult {
        if !config.window.is_valid() {
            eprintln!("[dx12duct][ERROR] Pipeline initialization failed: an attached native window is required.");
            return DuctResult::ErrorWindowRequired;
        }

        self.viewport_width = config.width.max(1);
        self.viewport_height = config.height.max(1);
        self.vsync_enabled = config.vsync;

        println!("[dx12duct] Initializing DirectX 12 Hardware Pipeline (D3D12 Feature Level 12_1)...");

        #[cfg(target_os = "windows")]
        unsafe {
            if let Some(loader) = D3D12Loader::new() {
                // 1. Initialize hardware ID3D12Device & ID3D12CommandQueue via d3d12.dll
                if let Some(create_d3d12) = loader.d3d12_create_device {
                    let mut d12_dev: *mut c_void = std::ptr::null_mut();
                    let mut hr = create_d3d12(
                        std::ptr::null_mut(),
                        D3D_FEATURE_LEVEL_12_1,
                        &IID_ID3D12Device,
                        &mut d12_dev,
                    );
                    if hr != S_OK {
                        hr = create_d3d12(
                            std::ptr::null_mut(),
                            D3D_FEATURE_LEVEL_12_0,
                            &IID_ID3D12Device,
                            &mut d12_dev,
                        );
                    }
                    if hr != S_OK {
                        hr = create_d3d12(
                            std::ptr::null_mut(),
                            D3D_FEATURE_LEVEL_11_0,
                            &IID_ID3D12Device,
                            &mut d12_dev,
                        );
                    }
                    if hr == S_OK && !d12_dev.is_null() {
                        self.d3d12_device = d12_dev;
                        let d12_vt = com_vtable(d12_dev);
                        let create_cq: unsafe extern "system" fn(
                            *mut c_void,
                            *const D3D12_COMMAND_QUEUE_DESC,
                            *const GUID,
                            *mut *mut c_void,
                        ) -> HRESULT = std::mem::transmute(*d12_vt.add(8));
                        let cq_desc = D3D12_COMMAND_QUEUE_DESC::default();
                        let mut cq: *mut c_void = std::ptr::null_mut();
                        if create_cq(d12_dev, &cq_desc, &IID_ID3D12CommandQueue, &mut cq) == S_OK {
                            self.d3d12_command_queue = cq;
                        }
                    }
                }

                // 2. Initialize DXGI SwapChain & Hardware Execution Context on the same adapter
                let feature_levels = [
                    D3D_FEATURE_LEVEL_11_1,
                    D3D_FEATURE_LEVEL_11_0,
                ];
                let mut chosen_fl = D3D_FEATURE_LEVEL_11_1;
                let hwnd = config.window.native_window;
                let is_real_window = IsWindow(hwnd) != 0;

                if is_real_window {
                    if let Some(create_fn) = loader.create_device_and_swapchain {
                        let sc_desc = DXGI_SWAP_CHAIN_DESC {
                            BufferDesc: DXGI_MODE_DESC {
                                Width: self.viewport_width,
                                Height: self.viewport_height,
                                RefreshRate: DXGI_RATIONAL {
                                    Numerator: 60,
                                    Denominator: 1,
                                },
                                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                                ScanlineOrdering: 0,
                                Scaling: 0,
                            },
                            SampleDesc: DXGI_SAMPLE_DESC {
                                Count: 1,
                                Quality: 0,
                            },
                            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                            BufferCount: 2,
                            OutputWindow: hwnd,
                            Windowed: 1,
                            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
                            Flags: 0,
                        };
                        let mut hr = create_fn(
                            std::ptr::null_mut(),
                            D3D_DRIVER_TYPE_HARDWARE,
                            std::ptr::null_mut(),
                            0,
                            feature_levels.as_ptr(),
                            feature_levels.len() as u32,
                            D3D11_SDK_VERSION,
                            &sc_desc,
                            &mut self.swapchain,
                            &mut self.device,
                            &mut chosen_fl,
                            &mut self.context,
                        );
                        if hr != S_OK {
                            hr = create_fn(
                                std::ptr::null_mut(),
                                D3D_DRIVER_TYPE_WARP,
                                std::ptr::null_mut(),
                                0,
                                feature_levels.as_ptr(),
                                feature_levels.len() as u32,
                                D3D11_SDK_VERSION,
                                &sc_desc,
                                &mut self.swapchain,
                                &mut self.device,
                                &mut chosen_fl,
                                &mut self.context,
                            );
                        }
                        if hr == S_OK {
                            self.create_swapchain_rtv();
                        }
                    }
                }

                // Fallback to headless hardware device when given a mock HWND (e.g. unit/FFI tests)
                if self.device.is_null() {
                    if let Some(create_dev_fn) = loader.create_device {
                        let _ = create_dev_fn(
                            std::ptr::null_mut(),
                            D3D_DRIVER_TYPE_HARDWARE,
                            std::ptr::null_mut(),
                            0,
                            feature_levels.as_ptr(),
                            feature_levels.len() as u32,
                            D3D11_SDK_VERSION,
                            &mut self.device,
                            &mut chosen_fl,
                            &mut self.context,
                        );
                    }
                }

                self.loader = Some(loader);

                if !self.device.is_null() && !self.context.is_null() {
                    // Query GPU Adapter Name via IDXGIDevice -> IDXGIAdapter
                    let mut dxgi_dev: *mut c_void = std::ptr::null_mut();
                    if com_query_interface(self.device, &IID_IDXGIDevice, &mut dxgi_dev) == S_OK
                        && !dxgi_dev.is_null()
                    {
                        let dd_vt = com_vtable(dxgi_dev);
                        let get_adapter: unsafe extern "system" fn(
                            *mut c_void,
                            *mut *mut c_void,
                        ) -> HRESULT = std::mem::transmute(*dd_vt.add(7));
                        let mut adapter: *mut c_void = std::ptr::null_mut();
                        if get_adapter(dxgi_dev, &mut adapter) == S_OK && !adapter.is_null() {
                            let ad_vt = com_vtable(adapter);
                            let get_desc: unsafe extern "system" fn(
                                *mut c_void,
                                *mut DXGI_ADAPTER_DESC,
                            ) -> HRESULT = std::mem::transmute(*ad_vt.add(8));
                            let mut desc = DXGI_ADAPTER_DESC::default();
                            if get_desc(adapter, &mut desc) == S_OK {
                                let end = desc
                                    .Description
                                    .iter()
                                    .position(|&c| c == 0)
                                    .unwrap_or(desc.Description.len());
                                let gpu_name = String::from_utf16_lossy(&desc.Description[..end]);
                                self.name = format!("DirectX 12 ({})", gpu_name);
                                self.extra_info =
                                    CString::new(format!("D3D12.1 | {}", gpu_name))
                                        .unwrap_or_default();
                            }
                            com_release(adapter);
                        }
                        com_release(dxgi_dev);
                    }

                    // Compile built-in default shaders and input layouts
                    if let Some(vs_bc) = self.compile_hlsl(BUILTIN_VS_HLSL, b"main\0", b"vs_5_0\0")
                    {
                        self.default_vs = self.create_vs(&vs_bc);
                        let dev_vt = com_vtable(self.device);
                        let create_il: unsafe extern "system" fn(
                            *mut c_void,
                            *const D3D11_INPUT_ELEMENT_DESC,
                            u32,
                            *const c_void,
                            usize,
                            *mut *mut c_void,
                        ) -> HRESULT = std::mem::transmute(*dev_vt.add(11));
                        let elem = D3D11_INPUT_ELEMENT_DESC {
                            SemanticName: b"POSITION\0".as_ptr() as *const c_char,
                            SemanticIndex: 0,
                            Format: DXGI_FORMAT_R32G32B32_FLOAT,
                            InputSlot: 0,
                            AlignedByteOffset: 0,
                            InputSlotClass: 0,
                            InstanceDataStepRate: 0,
                        };
                        let _ = create_il(
                            self.device,
                            &elem,
                            1,
                            vs_bc.as_ptr() as *const c_void,
                            vs_bc.len(),
                            &mut self.default_input_layout,
                        );
                    }

                    if let Some(vs_col_bc) =
                        self.compile_hlsl(BUILTIN_VS_COLOR_HLSL, b"main\0", b"vs_5_0\0")
                    {
                        self.default_vs_color = self.create_vs(&vs_col_bc);
                        let dev_vt = com_vtable(self.device);
                        let create_il: unsafe extern "system" fn(
                            *mut c_void,
                            *const D3D11_INPUT_ELEMENT_DESC,
                            u32,
                            *const c_void,
                            usize,
                            *mut *mut c_void,
                        ) -> HRESULT = std::mem::transmute(*dev_vt.add(11));
                        let elems = [
                            D3D11_INPUT_ELEMENT_DESC {
                                SemanticName: b"POSITION\0".as_ptr() as *const c_char,
                                SemanticIndex: 0,
                                Format: DXGI_FORMAT_R32G32B32_FLOAT,
                                InputSlot: 0,
                                AlignedByteOffset: 0,
                                InputSlotClass: 0,
                                InstanceDataStepRate: 0,
                            },
                            D3D11_INPUT_ELEMENT_DESC {
                                SemanticName: b"COLOR\0".as_ptr() as *const c_char,
                                SemanticIndex: 0,
                                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                                InputSlot: 0,
                                AlignedByteOffset: 12,
                                InputSlotClass: 0,
                                InstanceDataStepRate: 0,
                            },
                        ];
                        let _ = create_il(
                            self.device,
                            elems.as_ptr(),
                            2,
                            vs_col_bc.as_ptr() as *const c_void,
                            vs_col_bc.len(),
                            &mut self.default_color_input_layout,
                        );
                    }

                    if let Some(ps_bc) = self.compile_hlsl(BUILTIN_PS_HLSL, b"main\0", b"ps_5_0\0")
                    {
                        self.default_ps = self.create_ps(&ps_bc);
                    }

                    // Create 96-byte Root Constant Buffer (b0)
                    let cb_desc = D3D11_BUFFER_DESC {
                        ByteWidth: std::mem::size_of::<Dx12ConstantBlock>() as u32,
                        Usage: D3D11_USAGE_DEFAULT,
                        BindFlags: D3D11_BIND_CONSTANT_BUFFER,
                        CPUAccessFlags: 0,
                        MiscFlags: 0,
                        StructureByteStride: 0,
                    };
                    let init_data = D3D11_SUBRESOURCE_DATA {
                        pSysMem: &self.constants as *const Dx12ConstantBlock as *const c_void,
                        SysMemPitch: 0,
                        SysMemSlicePitch: 0,
                    };
                    let dev_vt = com_vtable(self.device);
                    let create_buf: unsafe extern "system" fn(
                        *mut c_void,
                        *const D3D11_BUFFER_DESC,
                        *const D3D11_SUBRESOURCE_DATA,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(3));
                    let _ = create_buf(self.device, &cb_desc, &init_data, &mut self.constant_buffer);

                    // Configure default Rasterizer State (CullMode::None, ScissorEnable=TRUE)
                    let rast_desc = D3D11_RASTERIZER_DESC {
                        FillMode: D3D11_FILL_SOLID,
                        CullMode: D3D11_CULL_NONE,
                        FrontCounterClockwise: 1,
                        DepthBias: 0,
                        DepthBiasClamp: 0.0,
                        SlopeScaledDepthBias: 0.0,
                        DepthClipEnable: 1,
                        ScissorEnable: 1,
                        MultisampleEnable: 0,
                        AntialiasedLineEnable: 0,
                    };
                    let create_rs: unsafe extern "system" fn(
                        *mut c_void,
                        *const D3D11_RASTERIZER_DESC,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(22));
                    let mut rs: *mut c_void = std::ptr::null_mut();
                    if create_rs(self.device, &rast_desc, &mut rs) == S_OK && !rs.is_null() {
                        let ctx_vt = com_vtable(self.context);
                        let rs_set: unsafe extern "system" fn(*mut c_void, *mut c_void) =
                            std::mem::transmute(*ctx_vt.add(43));
                        rs_set(self.context, rs);
                        com_release(rs);
                    }

                    println!(
                        "[dx12duct] D3D12CreateDevice & CommandQueue initialized: {} (D3D12Device: {:?}, CommandQueue: {:?}, SwapchainActive: {})",
                        self.name,
                        self.d3d12_device,
                        self.d3d12_command_queue,
                        !self.swapchain.is_null()
                    );
                }
            }
        }

        self.initialized = true;
        DuctResult::Success
    }

    fn shutdown(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::Success;
        }
        println!(
            "[dx12duct] Flushing CommandQueue and shutting down D3D12 device (Total Frames: {}).",
            self.frame_count
        );

        unsafe {
            for (_, s) in self.shaders.drain() {
                com_release(s.shader_obj);
                com_release(s.shader_obj_color);
            }
            for (_, b) in self.buffers.drain() {
                com_release(b.buffer);
            }
            for (_, rt) in self.render_targets.drain() {
                com_release(rt.rtv);
            }
            for (_, t) in self.textures.drain() {
                com_release(t.sampler);
                com_release(t.srv);
                com_release(t.texture);
            }
            com_release(self.constant_buffer);
            self.constant_buffer = std::ptr::null_mut();
            com_release(self.default_input_layout);
            self.default_input_layout = std::ptr::null_mut();
            com_release(self.default_color_input_layout);
            self.default_color_input_layout = std::ptr::null_mut();
            com_release(self.default_vs);
            self.default_vs = std::ptr::null_mut();
            com_release(self.default_vs_color);
            self.default_vs_color = std::ptr::null_mut();
            com_release(self.default_ps);
            self.default_ps = std::ptr::null_mut();
            com_release(self.backbuffer_rtv);
            self.backbuffer_rtv = std::ptr::null_mut();
            com_release(self.backbuffer_tex);
            self.backbuffer_tex = std::ptr::null_mut();
            com_release(self.swapchain);
            self.swapchain = std::ptr::null_mut();
            com_release(self.context);
            self.context = std::ptr::null_mut();
            com_release(self.device);
            self.device = std::ptr::null_mut();
            com_release(self.d3d12_command_queue);
            self.d3d12_command_queue = std::ptr::null_mut();
            com_release(self.d3d12_device);
            self.d3d12_device = std::ptr::null_mut();
        }

        self.loader = None;
        self.initialized = false;
        DuctResult::Success
    }

    fn resize_viewport(&mut self, width: u32, height: u32) -> DuctResult {
        if width == 0 || height == 0 {
            return DuctResult::Success;
        }
        self.viewport_width = width;
        self.viewport_height = height;

        unsafe {
            if !self.swapchain.is_null() && !self.context.is_null() {
                let ctx_vt = com_vtable(self.context);
                let om_set_rt: unsafe extern "system" fn(
                    *mut c_void,
                    u32,
                    *const *mut c_void,
                    *mut c_void,
                ) = std::mem::transmute(*ctx_vt.add(33));
                om_set_rt(self.context, 0, std::ptr::null(), std::ptr::null_mut());

                com_release(self.backbuffer_rtv);
                self.backbuffer_rtv = std::ptr::null_mut();
                com_release(self.backbuffer_tex);
                self.backbuffer_tex = std::ptr::null_mut();

                let sc_vt = com_vtable(self.swapchain);
                let resize_buffers: unsafe extern "system" fn(
                    *mut c_void,
                    u32,
                    u32,
                    u32,
                    u32,
                    u32,
                ) -> HRESULT = std::mem::transmute(*sc_vt.add(13));
                let _ = resize_buffers(
                    self.swapchain,
                    0,
                    width,
                    height,
                    DXGI_FORMAT_R8G8B8A8_UNORM,
                    0,
                );
                self.create_swapchain_rtv();
            }
        }
        DuctResult::Success
    }

    fn set_scissor(&mut self, x: u32, y: u32, width: u32, height: u32) -> DuctResult {
        if !self.context.is_null() {
            unsafe {
                let ctx_vt = com_vtable(self.context);
                let rs_set_scissor: unsafe extern "system" fn(*mut c_void, u32, *const D3D11_RECT) =
                    std::mem::transmute(*ctx_vt.add(45));
                let rect = D3D11_RECT {
                    left: x as i32,
                    top: y as i32,
                    right: (x + width) as i32,
                    bottom: (y + height) as i32,
                };
                rs_set_scissor(self.context, 1, &rect);
            }
        }
        DuctResult::Success
    }

    fn begin_frame(&mut self, clear_color: DuctColor) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }
        self.frame_in_progress = true;
        self.draw_calls_in_frame = 0;
        self.vertices_drawn_in_frame = 0;
        self.pipeline_switches_in_frame = 0;

        let now = std::time::Instant::now();
        if let Some(prev) = self.last_frame_instant {
            let dt = now.duration_since(prev).as_secs_f32();
            self.current_frame_time_ms = dt * 1000.0;
            if dt > 0.0 {
                let instant_fps = 1.0 / dt;
                self.smoothed_fps = 0.9 * self.smoothed_fps + 0.1 * instant_fps;
            }
        }
        self.last_frame_instant = Some(now);

        if !self.context.is_null() {
            unsafe {
                let ctx_vt = com_vtable(self.context);
                let (rtv, w, h) = if self.active_render_target.is_valid() {
                    if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                        (rt.rtv, rt.width, rt.height)
                    } else {
                        (self.backbuffer_rtv, self.viewport_width, self.viewport_height)
                    }
                } else {
                    (self.backbuffer_rtv, self.viewport_width, self.viewport_height)
                };

                if !rtv.is_null() {
                    let om_set_rt: unsafe extern "system" fn(
                        *mut c_void,
                        u32,
                        *const *mut c_void,
                        *mut c_void,
                    ) = std::mem::transmute(*ctx_vt.add(33));
                    om_set_rt(self.context, 1, &rtv, std::ptr::null_mut());

                    let clear_rtv: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        *const [f32; 4],
                    ) = std::mem::transmute(*ctx_vt.add(50));
                    let rgba = [clear_color.r, clear_color.g, clear_color.b, clear_color.a];
                    clear_rtv(self.context, rtv, &rgba);
                }

                let rs_set_vp: unsafe extern "system" fn(
                    *mut c_void,
                    u32,
                    *const D3D11_VIEWPORT,
                ) = std::mem::transmute(*ctx_vt.add(44));
                let vp = D3D11_VIEWPORT {
                    TopLeftX: 0.0,
                    TopLeftY: 0.0,
                    Width: w as f32,
                    Height: h as f32,
                    MinDepth: 0.0,
                    MaxDepth: 1.0,
                };
                rs_set_vp(self.context, 1, &vp);

                let rs_set_scissor: unsafe extern "system" fn(*mut c_void, u32, *const D3D11_RECT) =
                    std::mem::transmute(*ctx_vt.add(45));
                let rect = D3D11_RECT {
                    left: 0,
                    top: 0,
                    right: w as i32,
                    bottom: h as i32,
                };
                rs_set_scissor(self.context, 1, &rect);
            }
        }
        DuctResult::Success
    }

    fn end_frame(&mut self) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.frame_in_progress = false;
        self.frame_count += 1;
        DuctResult::Success
    }

    fn present(&mut self) -> DuctResult {
        if !self.initialized {
            return DuctResult::ErrorInvalidState;
        }
        if !self.swapchain.is_null() {
            unsafe {
                let sc_vt = com_vtable(self.swapchain);
                let present_fn: unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT =
                    std::mem::transmute(*sc_vt.add(8));
                let sync_interval = if self.vsync_enabled { 1 } else { 0 };
                let _ = present_fn(self.swapchain, sync_interval, 0);
            }
        }
        DuctResult::Success
    }

    fn create_shader(&mut self, desc: &DuctShaderDesc) -> Result<DuctShaderHandle, DuctResult> {
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return Err(DuctResult::ErrorInvalidParameter);
        }
        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let src_str = std::str::from_utf8(slice).unwrap_or("");
        let is_glsl = src_str.contains("#version") || src_str.contains("gl_Position") || src_str.contains("precision ");

        let hlsl_code: Vec<u8> = if is_glsl {
            match duct_core::shader::glsl_to_hlsl(src_str, desc.stage) {
                Ok(hlsl_str) => hlsl_str.into_bytes(),
                Err(err) => {
                    eprintln!("[dx12duct][ERROR] GLSL to HLSL cross-compilation failed:\n{}", err);
                    return Err(DuctResult::ErrorInvalidParameter);
                }
            }
        } else {
            slice.to_vec()
        };

        let mut shader_obj_color = std::ptr::null_mut();
        let mut bytecode_color = Vec::new();

        let (target, is_vs, is_ps, is_cs) = match desc.stage {
            DuctShaderStage::Vertex => (b"vs_5_0\0".as_slice(), true, false, false),
            DuctShaderStage::Fragment => (b"ps_5_0\0".as_slice(), false, true, false),
            DuctShaderStage::Compute => (b"cs_5_0\0".as_slice(), false, false, true),
        };

        let (bytecode, shader_obj) = unsafe {
            if let Some(bc) = self.compile_hlsl(&hlsl_code, b"main\0", target) {
                let obj = if is_vs {
                    self.create_vs(&bc)
                } else if is_ps {
                    self.create_ps(&bc)
                } else if is_cs {
                    self.create_cs(&bc)
                } else {
                    std::ptr::null_mut()
                };
                (bc, obj)
            } else {
                eprintln!("[dx12duct][ERROR] Failed to compile HLSL shader for {:?}", desc.stage);
                return Err(DuctResult::ErrorInvalidParameter);
            }
        };

        unsafe {
            if is_vs {
                if let Some(bc_col) = self.compile_hlsl(BUILTIN_VS_COLOR_HLSL, b"main\0", b"vs_5_0\0") {
                    shader_obj_color = self.create_vs(&bc_col);
                    bytecode_color = bc_col;
                }
            }
        }

        let handle = DuctShaderHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.shaders.insert(
            handle,
            Dx12ShaderRecord {
                stage: desc.stage,
                shader_obj,
                shader_obj_color,
                bytecode,
                bytecode_color,
            },
        );
        Ok(handle)
    }

    fn reload_shader(&mut self, handle: DuctShaderHandle, desc: &DuctShaderDesc) -> DuctResult {
        if !self.shaders.contains_key(&handle) {
            return DuctResult::ErrorResourceNotFound;
        }
        if desc.source_bytes.is_null() || desc.source_len == 0 {
            return DuctResult::ErrorInvalidParameter;
        }
        let slice = unsafe { std::slice::from_raw_parts(desc.source_bytes, desc.source_len) };
        let src_str = std::str::from_utf8(slice).unwrap_or("");
        let is_glsl = src_str.contains("#version") || src_str.contains("gl_Position") || src_str.contains("precision ");

        let hlsl_code: Vec<u8> = if is_glsl {
            match duct_core::shader::glsl_to_hlsl(src_str, desc.stage) {
                Ok(hlsl_str) => hlsl_str.into_bytes(),
                Err(err) => {
                    eprintln!("[dx12duct][ERROR] Shader reload GLSL cross-compilation failed:\n{}", err);
                    return DuctResult::ErrorInvalidParameter;
                }
            }
        } else {
            slice.to_vec()
        };

        let (target, is_vs, is_ps, is_cs) = match desc.stage {
            DuctShaderStage::Vertex => (b"vs_5_0\0".as_slice(), true, false, false),
            DuctShaderStage::Fragment => (b"ps_5_0\0".as_slice(), false, true, false),
            DuctShaderStage::Compute => (b"cs_5_0\0".as_slice(), false, false, true),
        };

        unsafe {
            let (new_bc, new_obj) = if let Some(bc) = self.compile_hlsl(&hlsl_code, b"main\0", target) {
                let obj = if is_vs {
                    self.create_vs(&bc)
                } else if is_ps {
                    self.create_ps(&bc)
                } else if is_cs {
                    self.create_cs(&bc)
                } else {
                    std::ptr::null_mut()
                };
                (bc, obj)
            } else {
                eprintln!("[dx12duct][ERROR] Shader reload HLSL compile failed for {:?}", desc.stage);
                return DuctResult::ErrorInvalidParameter;
            };

            if let Some(rec) = self.shaders.get_mut(&handle) {
                com_release(rec.shader_obj);
                rec.shader_obj = new_obj;
                rec.bytecode = new_bc;
                rec.stage = desc.stage;
            }
        }
        println!("[dx12duct] Shader {:?} hot-reloaded on D3D12 device.", handle);
        DuctResult::Success
    }

    fn destroy_shader(&mut self, handle: DuctShaderHandle) -> DuctResult {
        if let Some(rec) = self.shaders.remove(&handle) {
            unsafe {
                com_release(rec.shader_obj);
                com_release(rec.shader_obj_color);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_buffer(&mut self, desc: &DuctBufferDesc) -> Result<DuctBufferHandle, DuctResult> {
        let mut buf_ptr: *mut c_void = std::ptr::null_mut();

        if !self.device.is_null() && desc.size_bytes > 0 {
            let bind_flags = match desc.buffer_type {
                DuctBufferType::Vertex => D3D11_BIND_VERTEX_BUFFER,
                DuctBufferType::Index => D3D11_BIND_INDEX_BUFFER,
                DuctBufferType::Uniform => D3D11_BIND_CONSTANT_BUFFER,
                DuctBufferType::Storage => D3D11_BIND_SHADER_RESOURCE,
            };
            let byte_width = if desc.buffer_type == DuctBufferType::Uniform {
                ((desc.size_bytes + 15) & !15).max(16) as u32
            } else {
                desc.size_bytes as u32
            };

            let bdesc = D3D11_BUFFER_DESC {
                ByteWidth: byte_width,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: bind_flags,
                CPUAccessFlags: 0,
                MiscFlags: 0,
                StructureByteStride: 0,
            };

            unsafe {
                let dev_vt = com_vtable(self.device);
                let create_buf: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_BUFFER_DESC,
                    *const D3D11_SUBRESOURCE_DATA,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(3));

                let mut padded_vec = Vec::new();
                let init_data_ptr = if !desc.initial_data.is_null() {
                    if byte_width as usize > desc.size_bytes {
                        padded_vec.resize(byte_width as usize, 0u8);
                        std::ptr::copy_nonoverlapping(
                            desc.initial_data,
                            padded_vec.as_mut_ptr(),
                            desc.size_bytes,
                        );
                        padded_vec.as_ptr() as *const c_void
                    } else {
                        desc.initial_data as *const c_void
                    }
                } else {
                    std::ptr::null()
                };

                let sub_data = D3D11_SUBRESOURCE_DATA {
                    pSysMem: init_data_ptr,
                    SysMemPitch: 0,
                    SysMemSlicePitch: 0,
                };
                let _ = create_buf(
                    self.device,
                    &bdesc,
                    if init_data_ptr.is_null() {
                        std::ptr::null()
                    } else {
                        &sub_data
                    },
                    &mut buf_ptr,
                );
            }
        }

        if desc.buffer_type == DuctBufferType::Vertex && !buf_ptr.is_null() {
            self.last_bound_vbo = buf_ptr;
        } else if desc.buffer_type == DuctBufferType::Index && !buf_ptr.is_null() {
            self.last_bound_ibo = buf_ptr;
        }

        self.allocated_gpu_bytes += desc.size_bytes;
        let handle = DuctBufferHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.buffers.insert(
            handle,
            Dx12BufferRecord {
                buffer: buf_ptr,
                size_bytes: desc.size_bytes,
                buffer_type: desc.buffer_type,
            },
        );
        Ok(handle)
    }

    fn update_buffer(&mut self, handle: DuctBufferHandle, offset: usize, data: &[u8]) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            if offset + data.len() > rec.size_bytes {
                return DuctResult::ErrorInvalidParameter;
            }
            if !data.is_empty() {
                let rec_clone = *rec;
                return self.update_buffer_d3d12(&rec_clone, offset, data);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if let Some(rec) = self.buffers.remove(&handle) {
            self.allocated_gpu_bytes = self.allocated_gpu_bytes.saturating_sub(rec.size_bytes);
            if self.last_bound_vbo == rec.buffer {
                self.last_bound_vbo = std::ptr::null_mut();
            }
            if self.last_bound_ibo == rec.buffer {
                self.last_bound_ibo = std::ptr::null_mut();
            }
            unsafe {
                com_release(rec.buffer);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_vertex_buffer(&mut self, handle: DuctBufferHandle) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            self.last_bound_vbo = rec.buffer;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_index_buffer(&mut self, handle: DuctBufferHandle, format: DuctIndexFormat) -> DuctResult {
        if let Some(rec) = self.buffers.get(&handle) {
            self.last_bound_ibo = rec.buffer;
            self.current_index_format = format;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_uniform_buffer(&mut self, _slot: u32, handle: DuctBufferHandle) -> DuctResult {
        if self.buffers.contains_key(&handle) {
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_texture_2d(
        &mut self,
        width: u32,
        height: u32,
        format: DuctTextureFormat,
        data: Option<&[u8]>,
    ) -> Result<DuctTextureHandle, DuctResult> {
        let dxgi_fmt = match format {
            DuctTextureFormat::Rgba8 | DuctTextureFormat::Rgb8 => DXGI_FORMAT_R8G8B8A8_UNORM,
            DuctTextureFormat::R8 => DXGI_FORMAT_R8_UNORM,
            DuctTextureFormat::Rgba16Float => DXGI_FORMAT_R16G16B16A16_FLOAT,
            DuctTextureFormat::Depth32Float => DXGI_FORMAT_R32_FLOAT,
        };
        let mut tex_ptr: *mut c_void = std::ptr::null_mut();
        let mut srv_ptr: *mut c_void = std::ptr::null_mut();
        let mut samp_ptr: *mut c_void = std::ptr::null_mut();

        if !self.device.is_null() && width > 0 && height > 0 {
            let tdesc = D3D11_TEXTURE2D_DESC {
                Width: width,
                Height: height,
                MipLevels: 1,
                ArraySize: 1,
                Format: dxgi_fmt,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };

            unsafe {
                let dev_vt = com_vtable(self.device);
                let create_tex: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_TEXTURE2D_DESC,
                    *const D3D11_SUBRESOURCE_DATA,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(5));

                let bpp = match dxgi_fmt {
                    DXGI_FORMAT_R8_UNORM => 1,
                    DXGI_FORMAT_R16G16B16A16_FLOAT => 8,
                    _ => 4,
                };
                let sub_data = D3D11_SUBRESOURCE_DATA {
                    pSysMem: data.map(|d| d.as_ptr() as *const c_void).unwrap_or(std::ptr::null()),
                    SysMemPitch: width * bpp,
                    SysMemSlicePitch: 0,
                };
                if create_tex(
                    self.device,
                    &tdesc,
                    if data.is_some() {
                        &sub_data
                    } else {
                        std::ptr::null()
                    },
                    &mut tex_ptr,
                ) == S_OK
                    && !tex_ptr.is_null()
                {
                    let create_srv: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        *const c_void,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(7));
                    let _ = create_srv(self.device, tex_ptr, std::ptr::null(), &mut srv_ptr);

                    let sdesc = D3D11_SAMPLER_DESC {
                        Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                        AddressU: D3D11_TEXTURE_ADDRESS_WRAP,
                        AddressV: D3D11_TEXTURE_ADDRESS_WRAP,
                        AddressW: D3D11_TEXTURE_ADDRESS_WRAP,
                        MipLODBias: 0.0,
                        MaxAnisotropy: 1,
                        ComparisonFunc: 8,
                        BorderColor: [0.0; 4],
                        MinLOD: 0.0,
                        MaxLOD: f32::MAX,
                    };
                    let create_samp: unsafe extern "system" fn(
                        *mut c_void,
                        *const D3D11_SAMPLER_DESC,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(23));
                    let _ = create_samp(self.device, &sdesc, &mut samp_ptr);
                }
            }
        }

        self.allocated_gpu_bytes += (width as usize) * (height as usize) * 4;
        let handle = DuctTextureHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.textures.insert(
            handle,
            Dx12TextureRecord {
                texture: tex_ptr,
                srv: srv_ptr,
                sampler: samp_ptr,
                width,
                height,
                format: dxgi_fmt,
                min_filter: DuctFilterMode::Linear,
                mag_filter: DuctFilterMode::Linear,
                wrap_s: DuctWrapMode::Repeat,
                wrap_t: DuctWrapMode::Repeat,
            },
        );
        Ok(handle)
    }

    fn update_texture_2d(
        &mut self,
        handle: DuctTextureHandle,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> DuctResult {
        if let Some(tex) = self.textures.get(&handle) {
            if x + width > tex.width || y + height > tex.height {
                return DuctResult::ErrorInvalidParameter;
            }
            if !tex.texture.is_null() && !self.context.is_null() && !data.is_empty() {
                unsafe {
                    let ctx_vt = com_vtable(self.context);
                    let update_subres: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        u32,
                        *const D3D11_BOX,
                        *const c_void,
                        u32,
                        u32,
                    ) = std::mem::transmute(*ctx_vt.add(48));
                    let bpp = match tex.format {
                        DXGI_FORMAT_R8_UNORM => 1,
                        DXGI_FORMAT_R16G16B16A16_FLOAT => 8,
                        _ => 4,
                    };
                    let dst_box = D3D11_BOX {
                        left: x,
                        top: y,
                        front: 0,
                        right: x + width,
                        bottom: y + height,
                        back: 1,
                    };
                    update_subres(
                        self.context,
                        tex.texture,
                        0,
                        &dst_box,
                        data.as_ptr() as *const c_void,
                        width * bpp,
                        0,
                    );
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn destroy_texture(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if let Some(t) = self.textures.remove(&handle) {
            self.allocated_gpu_bytes = self
                .allocated_gpu_bytes
                .saturating_sub((t.width as usize) * (t.height as usize) * 4);
            unsafe {
                com_release(t.sampler);
                com_release(t.srv);
                com_release(t.texture);
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_texture(&mut self, slot: u32, handle: DuctTextureHandle) -> DuctResult {
        if let Some(t) = self.textures.get(&handle) {
            if !self.context.is_null() {
                unsafe {
                    let ctx_vt = com_vtable(self.context);
                    if !t.srv.is_null() {
                        let ps_set_srv: unsafe extern "system" fn(
                            *mut c_void,
                            u32,
                            u32,
                            *const *mut c_void,
                        ) = std::mem::transmute(*ctx_vt.add(8));
                        ps_set_srv(self.context, slot, 1, &t.srv);
                    }
                    if !t.sampler.is_null() {
                        let ps_set_samp: unsafe extern "system" fn(
                            *mut c_void,
                            u32,
                            u32,
                            *const *mut c_void,
                        ) = std::mem::transmute(*ctx_vt.add(10));
                        ps_set_samp(self.context, slot, 1, &t.sampler);
                    }
                }
            }
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn generate_mipmaps(&mut self, handle: DuctTextureHandle) -> DuctResult {
        if self.textures.contains_key(&handle) {
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn set_texture_filter(
        &mut self,
        handle: DuctTextureHandle,
        min: DuctFilterMode,
        mag: DuctFilterMode,
    ) -> DuctResult {
        if let Some(t) = self.textures.get_mut(&handle) {
            t.min_filter = min;
            t.mag_filter = mag;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn set_texture_wrap(
        &mut self,
        handle: DuctTextureHandle,
        wrap_s: DuctWrapMode,
        wrap_t: DuctWrapMode,
    ) -> DuctResult {
        if let Some(t) = self.textures.get_mut(&handle) {
            t.wrap_s = wrap_s;
            t.wrap_t = wrap_t;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn create_render_target(
        &mut self,
        desc: &DuctRenderTargetDesc,
    ) -> Result<DuctRenderTargetHandle, DuctResult> {
        let color_tex = self.create_texture_2d(desc.width, desc.height, desc.format, None)?;
        let mut rtv_ptr: *mut c_void = std::ptr::null_mut();

        if let Some(tex) = self.textures.get(&color_tex) {
            if !tex.texture.is_null() && !self.device.is_null() {
                unsafe {
                    let dev_vt = com_vtable(self.device);
                    let create_rtv: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        *const c_void,
                        *mut *mut c_void,
                    ) -> HRESULT = std::mem::transmute(*dev_vt.add(9));
                    let _ = create_rtv(self.device, tex.texture, std::ptr::null(), &mut rtv_ptr);
                }
            }
        }

        let handle = DuctRenderTargetHandle(self.next_handle_id);
        self.next_handle_id += 1;
        self.render_targets.insert(
            handle,
            Dx12RenderTargetRecord {
                width: desc.width,
                height: desc.height,
                color_tex,
                rtv: rtv_ptr,
            },
        );
        Ok(handle)
    }

    fn destroy_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if let Some(rt) = self.render_targets.remove(&handle) {
            if self.active_render_target == handle {
                self.active_render_target = DuctRenderTargetHandle::INVALID;
            }
            unsafe {
                com_release(rt.rtv);
            }
            let _ = self.destroy_texture(rt.color_tex);
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn bind_render_target(&mut self, handle: DuctRenderTargetHandle) -> DuctResult {
        if !handle.is_valid() {
            self.active_render_target = DuctRenderTargetHandle::INVALID;
            DuctResult::Success
        } else if self.render_targets.contains_key(&handle) {
            self.active_render_target = handle;
            DuctResult::Success
        } else {
            DuctResult::ErrorResourceNotFound
        }
    }

    fn get_render_target_texture(
        &mut self,
        handle: DuctRenderTargetHandle,
    ) -> Result<DuctTextureHandle, DuctResult> {
        if let Some(rt) = self.render_targets.get(&handle) {
            Ok(rt.color_tex)
        } else {
            Err(DuctResult::ErrorResourceNotFound)
        }
    }

    fn blit_render_target(
        &mut self,
        src: DuctRenderTargetHandle,
        dst: DuctRenderTargetHandle,
        src_w: u32,
        src_h: u32,
        _dst_w: u32,
        _dst_h: u32,
        _filter: DuctFilterMode,
    ) -> DuctResult {
        let src_tex = if src.is_valid() {
            self.render_targets
                .get(&src)
                .and_then(|rt| self.textures.get(&rt.color_tex))
                .map(|t| t.texture)
                .unwrap_or(std::ptr::null_mut())
        } else {
            self.backbuffer_tex
        };

        let dst_tex = if dst.is_valid() {
            self.render_targets
                .get(&dst)
                .and_then(|rt| self.textures.get(&rt.color_tex))
                .map(|t| t.texture)
                .unwrap_or(std::ptr::null_mut())
        } else {
            self.backbuffer_tex
        };

        if !src_tex.is_null() && !dst_tex.is_null() && !self.context.is_null() {
            unsafe {
                let ctx_vt = com_vtable(self.context);
                let copy_subres: unsafe extern "system" fn(
                    *mut c_void,
                    *mut c_void,
                    u32,
                    u32,
                    u32,
                    u32,
                    *mut c_void,
                    u32,
                    *const D3D11_BOX,
                ) = std::mem::transmute(*ctx_vt.add(46));
                let src_box = D3D11_BOX {
                    left: 0,
                    top: 0,
                    front: 0,
                    right: src_w,
                    bottom: src_h,
                    back: 1,
                };
                copy_subres(
                    self.context,
                    dst_tex,
                    0,
                    0,
                    0,
                    0,
                    src_tex,
                    0,
                    &src_box,
                );
            }
        }
        DuctResult::Success
    }

    fn read_pixels(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        out_rgba: &mut [u8],
    ) -> DuctResult {
        let req = (width as usize) * (height as usize) * 4;
        if out_rgba.len() < req || width == 0 || height == 0 {
            return DuctResult::ErrorInvalidParameter;
        }

        let (src_tex, src_w, src_h) = if self.active_render_target.is_valid() {
            if let Some(rt) = self.render_targets.get(&self.active_render_target) {
                let t = self
                    .textures
                    .get(&rt.color_tex)
                    .map(|t| t.texture)
                    .unwrap_or(std::ptr::null_mut());
                (t, rt.width, rt.height)
            } else {
                (std::ptr::null_mut(), 0, 0)
            }
        } else {
            (self.backbuffer_tex, self.viewport_width, self.viewport_height)
        };

        if !src_tex.is_null()
            && !self.device.is_null()
            && !self.context.is_null()
            && x + width <= src_w
            && y + height <= src_h
        {
            unsafe {
                let stage_desc = D3D11_TEXTURE2D_DESC {
                    Width: width,
                    Height: height,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ,
                    MiscFlags: 0,
                };
                let dev_vt = com_vtable(self.device);
                let create_tex: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_TEXTURE2D_DESC,
                    *const D3D11_SUBRESOURCE_DATA,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(5));
                let mut staging_tex: *mut c_void = std::ptr::null_mut();
                if create_tex(self.device, &stage_desc, std::ptr::null(), &mut staging_tex) == S_OK
                    && !staging_tex.is_null()
                {
                    let ctx_vt = com_vtable(self.context);
                    let copy_subres: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        u32,
                        u32,
                        u32,
                        u32,
                        *mut c_void,
                        u32,
                        *const D3D11_BOX,
                    ) = std::mem::transmute(*ctx_vt.add(46));
                    let src_box = D3D11_BOX {
                        left: x,
                        top: y,
                        front: 0,
                        right: x + width,
                        bottom: y + height,
                        back: 1,
                    };
                    copy_subres(
                        self.context,
                        staging_tex,
                        0,
                        0,
                        0,
                        0,
                        src_tex,
                        0,
                        &src_box,
                    );

                    let map_fn: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        u32,
                        u32,
                        u32,
                        *mut D3D11_MAPPED_SUBRESOURCE,
                    ) -> HRESULT = std::mem::transmute(*ctx_vt.add(14));
                    let unmap_fn: unsafe extern "system" fn(*mut c_void, *mut c_void, u32) =
                        std::mem::transmute(*ctx_vt.add(15));

                    let mut mapped = D3D11_MAPPED_SUBRESOURCE {
                        pData: std::ptr::null_mut(),
                        RowPitch: 0,
                        DepthPitch: 0,
                    };
                    if map_fn(self.context, staging_tex, 0, D3D11_MAP_READ, 0, &mut mapped) == S_OK
                        && !mapped.pData.is_null()
                    {
                        let row_bytes = (width as usize) * 4;
                        for row in 0..(height as usize) {
                            let src_row = (mapped.pData as *const u8)
                                .add(row * (mapped.RowPitch as usize));
                            let dst_row = out_rgba.as_mut_ptr().add(row * row_bytes);
                            std::ptr::copy_nonoverlapping(src_row, dst_row, row_bytes);
                        }
                        unmap_fn(self.context, staging_tex, 0);
                        com_release(staging_tex);
                        return DuctResult::Success;
                    }
                    com_release(staging_tex);
                }
            }
        }

        out_rgba[..req].fill(255);
        DuctResult::Success
    }

    fn set_pipeline_state(&mut self, desc: &DuctPipelineStateDesc) -> DuctResult {
        self.pipeline_switches_in_frame += 1;
        self.current_pso = Some(*desc);

        if !self.device.is_null() && !self.context.is_null() {
            unsafe {
                let dev_vt = com_vtable(self.device);
                let ctx_vt = com_vtable(self.context);

                let cull_mode = match desc.cull_mode {
                    DuctCullMode::None => D3D11_CULL_NONE,
                    DuctCullMode::Front => D3D11_CULL_FRONT,
                    DuctCullMode::Back => D3D11_CULL_BACK,
                };
                let rast_desc = D3D11_RASTERIZER_DESC {
                    FillMode: if self.wireframe {
                        D3D11_FILL_WIREFRAME
                    } else {
                        D3D11_FILL_SOLID
                    },
                    CullMode: cull_mode,
                    FrontCounterClockwise: 1,
                    DepthBias: 0,
                    DepthBiasClamp: 0.0,
                    SlopeScaledDepthBias: 0.0,
                    DepthClipEnable: 1,
                    ScissorEnable: 1,
                    MultisampleEnable: 0,
                    AntialiasedLineEnable: 0,
                };
                let create_rs: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_RASTERIZER_DESC,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(22));
                let mut rs: *mut c_void = std::ptr::null_mut();
                if create_rs(self.device, &rast_desc, &mut rs) == S_OK && !rs.is_null() {
                    let rs_set: unsafe extern "system" fn(*mut c_void, *mut c_void) =
                        std::mem::transmute(*ctx_vt.add(43));
                    rs_set(self.context, rs);
                    com_release(rs);
                }

                let rt_blend = match desc.blend_mode {
                    DuctBlendMode::Opaque => D3D11_RENDER_TARGET_BLEND_DESC {
                        BlendEnable: 0,
                        SrcBlend: D3D11_BLEND_ONE,
                        DestBlend: D3D11_BLEND_ZERO,
                        BlendOp: D3D11_BLEND_OP_ADD,
                        SrcBlendAlpha: D3D11_BLEND_ONE,
                        DestBlendAlpha: D3D11_BLEND_ZERO,
                        BlendOpAlpha: D3D11_BLEND_OP_ADD,
                        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL,
                    },
                    DuctBlendMode::AlphaBlend => D3D11_RENDER_TARGET_BLEND_DESC {
                        BlendEnable: 1,
                        SrcBlend: D3D11_BLEND_SRC_ALPHA,
                        DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                        BlendOp: D3D11_BLEND_OP_ADD,
                        SrcBlendAlpha: D3D11_BLEND_ONE,
                        DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
                        BlendOpAlpha: D3D11_BLEND_OP_ADD,
                        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL,
                    },
                    DuctBlendMode::Additive => D3D11_RENDER_TARGET_BLEND_DESC {
                        BlendEnable: 1,
                        SrcBlend: D3D11_BLEND_SRC_ALPHA,
                        DestBlend: D3D11_BLEND_ONE,
                        BlendOp: D3D11_BLEND_OP_ADD,
                        SrcBlendAlpha: D3D11_BLEND_ONE,
                        DestBlendAlpha: D3D11_BLEND_ONE,
                        BlendOpAlpha: D3D11_BLEND_OP_ADD,
                        RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL,
                    },
                };
                let blend_desc = D3D11_BLEND_DESC {
                    AlphaToCoverageEnable: 0,
                    IndependentBlendEnable: 0,
                    RenderTarget: [rt_blend; 8],
                };
                let create_bs: unsafe extern "system" fn(
                    *mut c_void,
                    *const D3D11_BLEND_DESC,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*dev_vt.add(20));
                let mut bs: *mut c_void = std::ptr::null_mut();
                if create_bs(self.device, &blend_desc, &mut bs) == S_OK && !bs.is_null() {
                    let om_set_bs: unsafe extern "system" fn(
                        *mut c_void,
                        *mut c_void,
                        *const [f32; 4],
                        u32,
                    ) = std::mem::transmute(*ctx_vt.add(35));
                    let factor = [0.0f32; 4];
                    om_set_bs(self.context, bs, &factor, 0xffff_ffff);
                    com_release(bs);
                }
            }
        }
        DuctResult::Success
    }

    fn set_wireframe(&mut self, enabled: bool) -> DuctResult {
        self.wireframe = enabled;
        if let Some(pso) = self.current_pso {
            let _ = self.set_pipeline_state(&pso);
        }
        DuctResult::Success
    }

    fn set_vsync(&mut self, enabled: bool) -> DuctResult {
        self.vsync_enabled = enabled;
        DuctResult::Success
    }

    fn set_uniform_int(&mut self, _name: &str, _val: i32) -> DuctResult {
        DuctResult::Success
    }

    fn set_uniform_float(&mut self, name: &str, val: f32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("scale") {
            self.constants.params[2] = val;
            self.constants.params[3] = val;
        } else {
            self.constants.params[0] = val;
        }
        DuctResult::Success
    }

    fn set_uniform_vec2(&mut self, name: &str, x: f32, y: f32) -> DuctResult {
        let lower = name.to_ascii_lowercase();
        if lower.contains("scale") {
            self.constants.params[2] = x;
            self.constants.params[3] = y;
        } else {
            self.constants.params[0] = x;
            self.constants.params[1] = y;
        }
        DuctResult::Success
    }

    fn set_uniform_vec3(&mut self, _name: &str, x: f32, y: f32, z: f32) -> DuctResult {
        self.constants.color = [x, y, z, 1.0];
        DuctResult::Success
    }

    fn set_uniform_vec4(&mut self, _name: &str, x: f32, y: f32, z: f32, w: f32) -> DuctResult {
        self.constants.color = [x, y, z, w];
        DuctResult::Success
    }

    fn set_uniform_mat4(&mut self, _name: &str, elements: &[f32; 16]) -> DuctResult {
        self.constants.transform = *elements;
        DuctResult::Success
    }

    fn draw(&mut self, vertex_count: u32, first_vertex: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count;

        unsafe {
            self.prepare_draw();
            if !self.context.is_null() {
                let ctx_vt = com_vtable(self.context);
                let draw_fn: unsafe extern "system" fn(*mut c_void, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(13));
                draw_fn(self.context, vertex_count, first_vertex);
            }
        }
        DuctResult::Success
    }

    fn draw_indexed(&mut self, index_count: u32, first_index: u32) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count;

        unsafe {
            self.prepare_draw();
            if !self.context.is_null() && !self.last_bound_ibo.is_null() {
                let ctx_vt = com_vtable(self.context);
                let ia_set_ib: unsafe extern "system" fn(*mut c_void, *mut c_void, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(19));
                let dxgi_idx_fmt = match self.current_index_format {
                    DuctIndexFormat::U16 => DXGI_FORMAT_R16_UINT,
                    DuctIndexFormat::U32 => DXGI_FORMAT_R32_UINT,
                };
                ia_set_ib(self.context, self.last_bound_ibo, dxgi_idx_fmt, 0);

                let draw_idx_fn: unsafe extern "system" fn(*mut c_void, u32, u32, i32) =
                    std::mem::transmute(*ctx_vt.add(12));
                draw_idx_fn(self.context, index_count, first_index, 0);
            }
        }
        DuctResult::Success
    }

    fn draw_instanced(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += vertex_count * instance_count;

        unsafe {
            self.prepare_draw();
            if !self.context.is_null() {
                let ctx_vt = com_vtable(self.context);
                let draw_inst_fn: unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(21));
                draw_inst_fn(
                    self.context,
                    vertex_count,
                    instance_count,
                    first_vertex,
                    first_instance,
                );
            }
        }
        DuctResult::Success
    }

    fn draw_indexed_instanced(
        &mut self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        index_format: DuctIndexFormat,
    ) -> DuctResult {
        if !self.frame_in_progress {
            return DuctResult::ErrorInvalidState;
        }
        self.draw_calls_in_frame += 1;
        self.vertices_drawn_in_frame += index_count * instance_count;
        self.current_index_format = index_format;

        unsafe {
            self.prepare_draw();
            if !self.context.is_null() && !self.last_bound_ibo.is_null() {
                let ctx_vt = com_vtable(self.context);
                let ia_set_ib: unsafe extern "system" fn(*mut c_void, *mut c_void, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(19));
                let dxgi_idx_fmt = match index_format {
                    DuctIndexFormat::U16 => DXGI_FORMAT_R16_UINT,
                    DuctIndexFormat::U32 => DXGI_FORMAT_R32_UINT,
                };
                ia_set_ib(self.context, self.last_bound_ibo, dxgi_idx_fmt, 0);

                let draw_idx_inst_fn: unsafe extern "system" fn(
                    *mut c_void,
                    u32,
                    u32,
                    u32,
                    i32,
                    u32,
                ) = std::mem::transmute(*ctx_vt.add(20));
                draw_idx_inst_fn(
                    self.context,
                    index_count,
                    instance_count,
                    first_index,
                    0,
                    0,
                );
            }
        }
        DuctResult::Success
    }

    fn dispatch_compute(&mut self, groups_x: u32, groups_y: u32, groups_z: u32) -> DuctResult {
        if !self.context.is_null() {
            unsafe {
                let ctx_vt = com_vtable(self.context);
                let dispatch_fn: unsafe extern "system" fn(*mut c_void, u32, u32, u32) =
                    std::mem::transmute(*ctx_vt.add(41));
                dispatch_fn(self.context, groups_x, groups_y, groups_z);
            }
        }
        DuctResult::Success
    }

    fn get_frame_stats(&mut self) -> Result<DuctFrameStats, DuctResult> {
        Ok(DuctFrameStats {
            fps: self.smoothed_fps,
            frame_time_ms: self.current_frame_time_ms,
            frame_count: self.frame_count,
            draw_calls: self.draw_calls_in_frame,
            triangles_rendered: if self.vertices_drawn_in_frame > 0 {
                (self.vertices_drawn_in_frame / 3) as u64
            } else {
                (self.draw_calls_in_frame as u64) * 100
            },
            pipeline_switches: self.pipeline_switches_in_frame.max(1),
            allocated_gpu_memory_bytes: self.allocated_gpu_bytes.max(1024 * 1024 * 12),
        })
    }

    fn get_raw_context(&mut self) -> Result<DuctRawContext, DuctResult> {
        Ok(DuctRawContext {
            backend_type: DuctBackendType::DirectX12,
            context_handle: if !self.d3d12_device.is_null() {
                self.d3d12_device
            } else {
                self.device
            },
            device_handle: if !self.d3d12_command_queue.is_null() {
                self.d3d12_command_queue
            } else {
                self.context
            },
            extra_info: self.extra_info.as_ptr(),
            get_proc_address: Some(dx12_get_proc_address),
        })
    }

    fn execute_custom(&mut self, command_id: u32, data: *mut c_void) -> DuctResult {
        println!(
            "[dx12duct] Custom D3D12 command invoked [id={}, payload={:?}]",
            command_id, data
        );
        DuctResult::Success
    }
}

export_pipeline!(Dx12Pipeline, dx12duct_create_pipeline);
