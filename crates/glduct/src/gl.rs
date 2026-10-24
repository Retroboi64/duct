//! Zero-dependency OpenGL function pointers and core constants for `glduct`.
//!
//! Provides direct hardware binding to OpenGL without any external crates.

#![allow(non_snake_case, dead_code)]

use std::ffi::{c_char, c_void};

// ── Core OpenGL Constants ───────────────────────────────────────────────────

pub const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
pub const GL_DEPTH_BUFFER_BIT: u32 = 0x0000_0100;
pub const GL_STENCIL_BUFFER_BIT: u32 = 0x0000_0400;

pub const GL_POINTS: u32 = 0x0000;
pub const GL_LINES: u32 = 0x0001;
pub const GL_LINE_LOOP: u32 = 0x0002;
pub const GL_LINE_STRIP: u32 = 0x0003;
pub const GL_TRIANGLES: u32 = 0x0004;
pub const GL_TRIANGLE_STRIP: u32 = 0x0005;
pub const GL_TRIANGLE_FAN: u32 = 0x0006;

pub const GL_CW: u32 = 0x0900;
pub const GL_CCW: u32 = 0x0901;

pub const GL_FALSE: u8 = 0;
pub const GL_TRUE: u8 = 1;

pub const GL_FLOAT: u32 = 0x1406;
pub const GL_UNSIGNED_INT: u32 = 0x1405;
pub const GL_UNSIGNED_BYTE: u32 = 0x1401;

pub const GL_ARRAY_BUFFER: u32 = 0x8892;
pub const GL_ELEMENT_ARRAY_BUFFER: u32 = 0x8893;
pub const GL_UNIFORM_BUFFER: u32 = 0x8A11;
pub const GL_SHADER_STORAGE_BUFFER: u32 = 0x90D2;

pub const GL_STATIC_DRAW: u32 = 0x88E4;
pub const GL_DYNAMIC_DRAW: u32 = 0x88E8;

pub const GL_VERTEX_SHADER: u32 = 0x8B31;
pub const GL_FRAGMENT_SHADER: u32 = 0x8B30;
pub const GL_COMPUTE_SHADER: u32 = 0x91B9;

pub const GL_COMPILE_STATUS: u32 = 0x8B81;
pub const GL_LINK_STATUS: u32 = 0x8B82;
pub const GL_INFO_LOG_LENGTH: u32 = 0x8B84;

pub const GL_BLEND: u32 = 0x0BE2;
pub const GL_SRC_ALPHA: u32 = 0x0302;
pub const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
pub const GL_ONE: u32 = 1;

pub const GL_DEPTH_TEST: u32 = 0x0B71;
pub const GL_LESS: u32 = 0x0201;

pub const GL_CULL_FACE: u32 = 0x0B44;
pub const GL_FRONT: u32 = 0x0404;
pub const GL_BACK: u32 = 0x0405;

pub const GL_SCISSOR_TEST: u32 = 0x0C11;

pub const GL_TEXTURE_2D: u32 = 0x0DE1;
pub const GL_RGBA: u32 = 0x1908;
pub const GL_RGB: u32 = 0x1907;
pub const GL_RED: u32 = 0x1903;
pub const GL_RGBA8: u32 = 0x8058;
pub const GL_RGB8: u32 = 0x8051;
pub const GL_R8: u32 = 0x8229;

pub const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
pub const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
pub const GL_TEXTURE_WRAP_S: u32 = 0x2802;
pub const GL_TEXTURE_WRAP_T: u32 = 0x2803;
pub const GL_LINEAR: u32 = 0x2601;
pub const GL_REPEAT: u32 = 0x2901;
pub const GL_TEXTURE0: u32 = 0x84C0;

pub const GL_FRONT_AND_BACK: u32 = 0x0408;
pub const GL_LINE: u32 = 0x1B01;
pub const GL_FILL: u32 = 0x1B02;

pub const GL_VENDOR: u32 = 0x1F00;
pub const GL_RENDERER: u32 = 0x1F01;
pub const GL_VERSION: u32 = 0x1F02;

// FBO & Renderbuffer Constants
pub const GL_FRAMEBUFFER: u32 = 0x8D40;
pub const GL_READ_FRAMEBUFFER: u32 = 0x8CA8;
pub const GL_DRAW_FRAMEBUFFER: u32 = 0x8CA9;
pub const GL_RENDERBUFFER: u32 = 0x8D41;
pub const GL_COLOR_ATTACHMENT0: u32 = 0x8CE0;
pub const GL_DEPTH_ATTACHMENT: u32 = 0x8D00;
pub const GL_DEPTH_STENCIL_ATTACHMENT: u32 = 0x821A;
pub const GL_DEPTH24_STENCIL8: u32 = 0x88F0;
pub const GL_FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;
pub const GL_FRAMEBUFFER_SRGB: u32 = 0x8DB9;
pub const GL_MULTISAMPLE: u32 = 0x809D;

// Pixel Store & Buffer Access
pub const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;
pub const GL_PACK_ALIGNMENT: u32 = 0x0D05;
pub const GL_READ_ONLY: u32 = 0x88B8;
pub const GL_WRITE_ONLY: u32 = 0x88B9;
pub const GL_READ_WRITE: u32 = 0x88BA;

// Blend Equations & Compute Barriers
pub const GL_FUNC_ADD: u32 = 0x8006;
pub const GL_FUNC_SUBTRACT: u32 = 0x800A;
pub const GL_FUNC_REVERSE_SUBTRACT: u32 = 0x800B;
pub const GL_MIN: u32 = 0x8007;
pub const GL_MAX: u32 = 0x8008;
pub const GL_ALL_BARRIER_BITS: u32 = 0xFFFF_FFFF;
pub const GL_SHADER_STORAGE_BARRIER_BIT: u32 = 0x0000_2000;
pub const GL_TEXTURE_FETCH_BARRIER_BIT: u32 = 0x0000_0008;
pub const GL_SHADER_IMAGE_ACCESS_BARRIER_BIT: u32 = 0x0000_0020;
pub const GL_STENCIL_TEST: u32 = 0x0B90;
pub const GL_POLYGON_OFFSET_FILL: u32 = 0x8037;

// Sampler & Mipmaps
pub const GL_NEAREST: i32 = 0x2600;
pub const GL_NEAREST_MIPMAP_NEAREST: i32 = 0x2700;
pub const GL_LINEAR_MIPMAP_NEAREST: i32 = 0x2701;
pub const GL_NEAREST_MIPMAP_LINEAR: i32 = 0x2702;
pub const GL_LINEAR_MIPMAP_LINEAR: i32 = 0x2703;
pub const GL_CLAMP_TO_EDGE: i32 = 0x812F;
pub const GL_MIRRORED_REPEAT: i32 = 0x8370;

// Data types
pub const GL_UNSIGNED_SHORT: u32 = 0x1403;

// Comparison functions
pub const GL_NEVER: u32 = 0x0200;
// GL_LESS is defined above as 0x0201
pub const GL_EQUAL: u32 = 0x0202;
pub const GL_LEQUAL: u32 = 0x0203;
pub const GL_GREATER: u32 = 0x0204;
pub const GL_NOTEQUAL: u32 = 0x0205;
pub const GL_GEQUAL: u32 = 0x0206;
pub const GL_ALWAYS: u32 = 0x0207;

// Blending factors
pub const GL_ZERO: u32 = 0;
// GL_ONE is defined above as 1
pub const GL_DST_COLOR: u32 = 0x0306;
pub const GL_ONE_MINUS_DST_COLOR: u32 = 0x0307;

// ── Function Pointer Table ──────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Gl {
    pub Viewport: unsafe extern "system" fn(x: i32, y: i32, width: i32, height: i32),
    pub Scissor: unsafe extern "system" fn(x: i32, y: i32, width: i32, height: i32),
    pub ClearColor: unsafe extern "system" fn(r: f32, g: f32, b: f32, a: f32),
    pub ClearDepth: unsafe extern "system" fn(depth: f64),
    pub ClearStencil: unsafe extern "system" fn(s: i32),
    pub Clear: unsafe extern "system" fn(mask: u32),
    pub Enable: unsafe extern "system" fn(cap: u32),
    pub Disable: unsafe extern "system" fn(cap: u32),
    pub BlendFunc: unsafe extern "system" fn(sfactor: u32, dfactor: u32),
    pub BlendFuncSeparate: unsafe extern "system" fn(sfactorRGB: u32, dfactorRGB: u32, sfactorAlpha: u32, dfactorAlpha: u32),
    pub BlendEquation: unsafe extern "system" fn(mode: u32),
    pub DepthFunc: unsafe extern "system" fn(func: u32),
    pub DepthMask: unsafe extern "system" fn(flag: u8),
    pub StencilFunc: unsafe extern "system" fn(func: u32, ref_: i32, mask: u32),
    pub StencilOp: unsafe extern "system" fn(fail: u32, zfail: u32, zpass: u32),
    pub StencilMask: unsafe extern "system" fn(mask: u32),
    pub ColorMask: unsafe extern "system" fn(red: u8, green: u8, blue: u8, alpha: u8),
    pub CullFace: unsafe extern "system" fn(mode: u32),
    pub FrontFace: unsafe extern "system" fn(mode: u32),
    pub PolygonMode: unsafe extern "system" fn(face: u32, mode: u32),
    pub PolygonOffset: unsafe extern "system" fn(factor: f32, units: f32),
    pub LineWidth: unsafe extern "system" fn(width: f32),
    pub PointSize: unsafe extern "system" fn(size: f32),
    pub PixelStorei: unsafe extern "system" fn(pname: u32, param: i32),
    pub GetString: unsafe extern "system" fn(name: u32) -> *const u8,
    pub GetError: unsafe extern "system" fn() -> u32,
    pub GetIntegerv: unsafe extern "system" fn(pname: u32, data: *mut i32),
    pub Finish: unsafe extern "system" fn(),
    pub Flush: unsafe extern "system" fn(),

    // VAO
    pub GenVertexArrays: unsafe extern "system" fn(n: i32, arrays: *mut u32),
    pub BindVertexArray: unsafe extern "system" fn(array: u32),
    pub DeleteVertexArrays: unsafe extern "system" fn(n: i32, arrays: *const u32),

    // Buffers
    pub GenBuffers: unsafe extern "system" fn(n: i32, buffers: *mut u32),
    pub BindBuffer: unsafe extern "system" fn(target: u32, buffer: u32),
    pub BindBufferBase: unsafe extern "system" fn(target: u32, index: u32, buffer: u32),
    pub BindBufferRange: unsafe extern "system" fn(target: u32, index: u32, buffer: u32, offset: isize, size: isize),
    pub BufferData: unsafe extern "system" fn(target: u32, size: isize, data: *const c_void, usage: u32),
    pub BufferSubData: unsafe extern "system" fn(target: u32, offset: isize, size: isize, data: *const c_void),
    pub MapBuffer: unsafe extern "system" fn(target: u32, access: u32) -> *mut c_void,
    pub UnmapBuffer: unsafe extern "system" fn(target: u32) -> u8,
    pub DeleteBuffers: unsafe extern "system" fn(n: i32, buffers: *const u32),

    // Vertex Attributes
    pub VertexAttribPointer: unsafe extern "system" fn(index: u32, size: i32, type_: u32, normalized: u8, stride: i32, pointer: *const c_void),
    pub VertexAttribIPointer: unsafe extern "system" fn(index: u32, size: i32, type_: u32, stride: i32, pointer: *const c_void),
    pub VertexAttribDivisor: unsafe extern "system" fn(index: u32, divisor: u32),
    pub EnableVertexAttribArray: unsafe extern "system" fn(index: u32),
    pub DisableVertexAttribArray: unsafe extern "system" fn(index: u32),

    // Shaders & Programs
    pub CreateShader: unsafe extern "system" fn(type_: u32) -> u32,
    pub ShaderSource: unsafe extern "system" fn(shader: u32, count: i32, string: *const *const c_char, length: *const i32),
    pub CompileShader: unsafe extern "system" fn(shader: u32),
    pub GetShaderiv: unsafe extern "system" fn(shader: u32, pname: u32, params: *mut i32),
    pub GetShaderInfoLog: unsafe extern "system" fn(shader: u32, bufSize: i32, length: *mut i32, infoLog: *mut c_char),
    pub DeleteShader: unsafe extern "system" fn(shader: u32),

    pub CreateProgram: unsafe extern "system" fn() -> u32,
    pub AttachShader: unsafe extern "system" fn(program: u32, shader: u32),
    pub LinkProgram: unsafe extern "system" fn(program: u32),
    pub GetProgramiv: unsafe extern "system" fn(program: u32, pname: u32, params: *mut i32),
    pub GetProgramInfoLog: unsafe extern "system" fn(program: u32, bufSize: i32, length: *mut i32, infoLog: *mut c_char),
    pub UseProgram: unsafe extern "system" fn(program: u32),
    pub DeleteProgram: unsafe extern "system" fn(program: u32),

    // Uniforms & Uniform Blocks
    pub GetUniformLocation: unsafe extern "system" fn(program: u32, name: *const c_char) -> i32,
    pub GetUniformBlockIndex: unsafe extern "system" fn(program: u32, uniformBlockName: *const c_char) -> u32,
    pub UniformBlockBinding: unsafe extern "system" fn(program: u32, uniformBlockIndex: u32, uniformBlockBinding: u32),
    pub Uniform1i: unsafe extern "system" fn(location: i32, v0: i32),
    pub Uniform2i: unsafe extern "system" fn(location: i32, v0: i32, v1: i32),
    pub Uniform3i: unsafe extern "system" fn(location: i32, v0: i32, v1: i32, v2: i32),
    pub Uniform4i: unsafe extern "system" fn(location: i32, v0: i32, v1: i32, v2: i32, v3: i32),
    pub Uniform1f: unsafe extern "system" fn(location: i32, v0: f32),
    pub Uniform2f: unsafe extern "system" fn(location: i32, v0: f32, v1: f32),
    pub Uniform3f: unsafe extern "system" fn(location: i32, v0: f32, v1: f32, v2: f32),
    pub Uniform4f: unsafe extern "system" fn(location: i32, v0: f32, v1: f32, v2: f32, v3: f32),
    pub Uniform1fv: unsafe extern "system" fn(location: i32, count: i32, value: *const f32),
    pub Uniform3fv: unsafe extern "system" fn(location: i32, count: i32, value: *const f32),
    pub Uniform4fv: unsafe extern "system" fn(location: i32, count: i32, value: *const f32),
    pub UniformMatrix3fv: unsafe extern "system" fn(location: i32, count: i32, transpose: u8, value: *const f32),
    pub UniformMatrix4fv: unsafe extern "system" fn(location: i32, count: i32, transpose: u8, value: *const f32),

    // Textures
    pub GenTextures: unsafe extern "system" fn(n: i32, textures: *mut u32),
    pub BindTexture: unsafe extern "system" fn(target: u32, texture: u32),
    pub TexImage2D: unsafe extern "system" fn(target: u32, level: i32, internalformat: i32, width: i32, height: i32, border: i32, format: u32, type_: u32, pixels: *const c_void),
    pub TexSubImage2D: unsafe extern "system" fn(target: u32, level: i32, xoffset: i32, yoffset: i32, width: i32, height: i32, format: u32, type_: u32, pixels: *const c_void),
    pub TexParameteri: unsafe extern "system" fn(target: u32, pname: u32, param: i32),
    pub TexParameterf: unsafe extern "system" fn(target: u32, pname: u32, param: f32),
    pub DeleteTextures: unsafe extern "system" fn(n: i32, textures: *const u32),
    pub ActiveTexture: unsafe extern "system" fn(texture: u32),
    pub GenerateMipmap: unsafe extern "system" fn(target: u32),

    // Framebuffers (Offscreen Render Targets), Blit & ReadPixels
    pub GenFramebuffers: unsafe extern "system" fn(n: i32, framebuffers: *mut u32),
    pub BindFramebuffer: unsafe extern "system" fn(target: u32, framebuffer: u32),
    pub DeleteFramebuffers: unsafe extern "system" fn(n: i32, framebuffers: *const u32),
    pub FramebufferTexture2D: unsafe extern "system" fn(target: u32, attachment: u32, textarget: u32, texture: u32, level: i32),
    pub GenRenderbuffers: unsafe extern "system" fn(n: i32, renderbuffers: *mut u32),
    pub BindRenderbuffer: unsafe extern "system" fn(target: u32, renderbuffer: u32),
    pub RenderbufferStorage: unsafe extern "system" fn(target: u32, internalformat: u32, width: i32, height: i32),
    pub FramebufferRenderbuffer: unsafe extern "system" fn(target: u32, attachment: u32, renderbuffertarget: u32, renderbuffer: u32),
    pub DeleteRenderbuffers: unsafe extern "system" fn(n: i32, renderbuffers: *const u32),
    pub CheckFramebufferStatus: unsafe extern "system" fn(target: u32) -> u32,
    pub BlitFramebuffer: unsafe extern "system" fn(srcX0: i32, srcY0: i32, srcX1: i32, srcY1: i32, dstX0: i32, dstY0: i32, dstX1: i32, dstY1: i32, mask: u32, filter: u32),
    pub ReadPixels: unsafe extern "system" fn(x: i32, y: i32, width: i32, height: i32, format: u32, type_: u32, pixels: *mut c_void),

    // Draw & Compute
    pub DrawArrays: unsafe extern "system" fn(mode: u32, first: i32, count: i32),
    pub DrawElements: unsafe extern "system" fn(mode: u32, count: i32, type_: u32, indices: *const c_void),
    pub DrawArraysInstanced: unsafe extern "system" fn(mode: u32, first: i32, count: i32, instancecount: i32),
    pub DrawElementsInstanced: unsafe extern "system" fn(mode: u32, count: i32, type_: u32, indices: *const c_void, instancecount: i32),
    pub DispatchCompute: unsafe extern "system" fn(num_groups_x: u32, num_groups_y: u32, num_groups_z: u32),
    pub MemoryBarrier: unsafe extern "system" fn(barriers: u32),
}

macro_rules! load_fn {
    ($loader:expr, $name:literal, $t:ty) => {{
        let ptr = $loader($name);
        if ptr.is_null() {
            panic!("Required OpenGL function '{}' could not be loaded", $name);
        }
        std::mem::transmute_copy::<$t, $t>(&std::mem::transmute(ptr))
    }};
    ($loader:expr, $name:literal, $t:ty, fallback: $fallback:expr) => {{
        let ptr = $loader($name);
        if ptr.is_null() {
            $fallback
        } else {
            std::mem::transmute_copy::<$t, $t>(&std::mem::transmute(ptr))
        }
    }};
}

unsafe extern "system" fn dummy_draw_instanced(_: u32, _: i32, _: i32, _: i32) {}
unsafe extern "system" fn dummy_draw_elements_instanced(_: u32, _: i32, _: u32, _: *const c_void, _: i32) {}
unsafe extern "system" fn dummy_generate_mipmap(_: u32) {}
unsafe extern "system" fn dummy_bind_buffer_base(_: u32, _: u32, _: u32) {}
unsafe extern "system" fn dummy_bind_buffer_range(_: u32, _: u32, _: u32, _: isize, _: isize) {}
unsafe extern "system" fn dummy_map_buffer(_: u32, _: u32) -> *mut c_void { std::ptr::null_mut() }
unsafe extern "system" fn dummy_unmap_buffer(_: u32) -> u8 { 0 }
unsafe extern "system" fn dummy_vertex_attrib_i_pointer(_: u32, _: i32, _: u32, _: i32, _: *const c_void) {}
unsafe extern "system" fn dummy_vertex_attrib_divisor(_: u32, _: u32) {}
unsafe extern "system" fn dummy_get_uniform_block_index(_: u32, _: *const c_char) -> u32 { 0xFFFF_FFFF }
unsafe extern "system" fn dummy_uniform_block_binding(_: u32, _: u32, _: u32) {}
unsafe extern "system" fn dummy_blit_framebuffer(_: i32, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32, _: u32, _: u32) {}
unsafe extern "system" fn dummy_dispatch_compute(_: u32, _: u32, _: u32) {}
unsafe extern "system" fn dummy_memory_barrier(_: u32) {}
unsafe extern "system" fn dummy_blend_func_separate(_: u32, _: u32, _: u32, _: u32) {}
unsafe extern "system" fn dummy_blend_equation(_: u32) {}

impl Gl {
    pub unsafe fn load_with<F: FnMut(&str) -> *const c_void>(mut loader: F) -> Self {
        unsafe {
            Self {
                Viewport: load_fn!(loader, "glViewport", unsafe extern "system" fn(i32, i32, i32, i32)),
                Scissor: load_fn!(loader, "glScissor", unsafe extern "system" fn(i32, i32, i32, i32)),
                ClearColor: load_fn!(loader, "glClearColor", unsafe extern "system" fn(f32, f32, f32, f32)),
                ClearDepth: load_fn!(loader, "glClearDepth", unsafe extern "system" fn(f64)),
                ClearStencil: load_fn!(loader, "glClearStencil", unsafe extern "system" fn(i32)),
                Clear: load_fn!(loader, "glClear", unsafe extern "system" fn(u32)),
                Enable: load_fn!(loader, "glEnable", unsafe extern "system" fn(u32)),
                Disable: load_fn!(loader, "glDisable", unsafe extern "system" fn(u32)),
                BlendFunc: load_fn!(loader, "glBlendFunc", unsafe extern "system" fn(u32, u32)),
                BlendFuncSeparate: load_fn!(loader, "glBlendFuncSeparate", unsafe extern "system" fn(u32, u32, u32, u32), fallback: dummy_blend_func_separate),
                BlendEquation: load_fn!(loader, "glBlendEquation", unsafe extern "system" fn(u32), fallback: dummy_blend_equation),
                DepthFunc: load_fn!(loader, "glDepthFunc", unsafe extern "system" fn(u32)),
                DepthMask: load_fn!(loader, "glDepthMask", unsafe extern "system" fn(u8)),
                StencilFunc: load_fn!(loader, "glStencilFunc", unsafe extern "system" fn(u32, i32, u32)),
                StencilOp: load_fn!(loader, "glStencilOp", unsafe extern "system" fn(u32, u32, u32)),
                StencilMask: load_fn!(loader, "glStencilMask", unsafe extern "system" fn(u32)),
                ColorMask: load_fn!(loader, "glColorMask", unsafe extern "system" fn(u8, u8, u8, u8)),
                CullFace: load_fn!(loader, "glCullFace", unsafe extern "system" fn(u32)),
                FrontFace: load_fn!(loader, "glFrontFace", unsafe extern "system" fn(u32)),
                PolygonMode: load_fn!(loader, "glPolygonMode", unsafe extern "system" fn(u32, u32)),
                PolygonOffset: load_fn!(loader, "glPolygonOffset", unsafe extern "system" fn(f32, f32)),
                LineWidth: load_fn!(loader, "glLineWidth", unsafe extern "system" fn(f32)),
                PointSize: load_fn!(loader, "glPointSize", unsafe extern "system" fn(f32)),
                PixelStorei: load_fn!(loader, "glPixelStorei", unsafe extern "system" fn(u32, i32)),
                GetString: load_fn!(loader, "glGetString", unsafe extern "system" fn(u32) -> *const u8),
                GetError: load_fn!(loader, "glGetError", unsafe extern "system" fn() -> u32),
                GetIntegerv: load_fn!(loader, "glGetIntegerv", unsafe extern "system" fn(u32, *mut i32)),
                Finish: load_fn!(loader, "glFinish", unsafe extern "system" fn()),
                Flush: load_fn!(loader, "glFlush", unsafe extern "system" fn()),

                GenVertexArrays: load_fn!(loader, "glGenVertexArrays", unsafe extern "system" fn(i32, *mut u32)),
                BindVertexArray: load_fn!(loader, "glBindVertexArray", unsafe extern "system" fn(u32)),
                DeleteVertexArrays: load_fn!(loader, "glDeleteVertexArrays", unsafe extern "system" fn(i32, *const u32)),

                GenBuffers: load_fn!(loader, "glGenBuffers", unsafe extern "system" fn(i32, *mut u32)),
                BindBuffer: load_fn!(loader, "glBindBuffer", unsafe extern "system" fn(u32, u32)),
                BindBufferBase: load_fn!(loader, "glBindBufferBase", unsafe extern "system" fn(u32, u32, u32), fallback: dummy_bind_buffer_base),
                BindBufferRange: load_fn!(loader, "glBindBufferRange", unsafe extern "system" fn(u32, u32, u32, isize, isize), fallback: dummy_bind_buffer_range),
                BufferData: load_fn!(loader, "glBufferData", unsafe extern "system" fn(u32, isize, *const c_void, u32)),
                BufferSubData: load_fn!(loader, "glBufferSubData", unsafe extern "system" fn(u32, isize, isize, *const c_void)),
                MapBuffer: load_fn!(loader, "glMapBuffer", unsafe extern "system" fn(u32, u32) -> *mut c_void, fallback: dummy_map_buffer),
                UnmapBuffer: load_fn!(loader, "glUnmapBuffer", unsafe extern "system" fn(u32) -> u8, fallback: dummy_unmap_buffer),
                DeleteBuffers: load_fn!(loader, "glDeleteBuffers", unsafe extern "system" fn(i32, *const u32)),

                VertexAttribPointer: load_fn!(loader, "glVertexAttribPointer", unsafe extern "system" fn(u32, i32, u32, u8, i32, *const c_void)),
                VertexAttribIPointer: load_fn!(loader, "glVertexAttribIPointer", unsafe extern "system" fn(u32, i32, u32, i32, *const c_void), fallback: dummy_vertex_attrib_i_pointer),
                VertexAttribDivisor: load_fn!(loader, "glVertexAttribDivisor", unsafe extern "system" fn(u32, u32), fallback: dummy_vertex_attrib_divisor),
                EnableVertexAttribArray: load_fn!(loader, "glEnableVertexAttribArray", unsafe extern "system" fn(u32)),
                DisableVertexAttribArray: load_fn!(loader, "glDisableVertexAttribArray", unsafe extern "system" fn(u32)),

                CreateShader: load_fn!(loader, "glCreateShader", unsafe extern "system" fn(u32) -> u32),
                ShaderSource: load_fn!(loader, "glShaderSource", unsafe extern "system" fn(u32, i32, *const *const c_char, *const i32)),
                CompileShader: load_fn!(loader, "glCompileShader", unsafe extern "system" fn(u32)),
                GetShaderiv: load_fn!(loader, "glGetShaderiv", unsafe extern "system" fn(u32, u32, *mut i32)),
                GetShaderInfoLog: load_fn!(loader, "glGetShaderInfoLog", unsafe extern "system" fn(u32, i32, *mut i32, *mut c_char)),
                DeleteShader: load_fn!(loader, "glDeleteShader", unsafe extern "system" fn(u32)),

                CreateProgram: load_fn!(loader, "glCreateProgram", unsafe extern "system" fn() -> u32),
                AttachShader: load_fn!(loader, "glAttachShader", unsafe extern "system" fn(u32, u32)),
                LinkProgram: load_fn!(loader, "glLinkProgram", unsafe extern "system" fn(u32)),
                GetProgramiv: load_fn!(loader, "glGetProgramiv", unsafe extern "system" fn(u32, u32, *mut i32)),
                GetProgramInfoLog: load_fn!(loader, "glGetProgramInfoLog", unsafe extern "system" fn(u32, i32, *mut i32, *mut c_char)),
                UseProgram: load_fn!(loader, "glUseProgram", unsafe extern "system" fn(u32)),
                DeleteProgram: load_fn!(loader, "glDeleteProgram", unsafe extern "system" fn(u32)),

                GetUniformLocation: load_fn!(loader, "glGetUniformLocation", unsafe extern "system" fn(u32, *const c_char) -> i32),
                GetUniformBlockIndex: load_fn!(loader, "glGetUniformBlockIndex", unsafe extern "system" fn(u32, *const c_char) -> u32, fallback: dummy_get_uniform_block_index),
                UniformBlockBinding: load_fn!(loader, "glUniformBlockBinding", unsafe extern "system" fn(u32, u32, u32), fallback: dummy_uniform_block_binding),
                Uniform1i: load_fn!(loader, "glUniform1i", unsafe extern "system" fn(i32, i32)),
                Uniform2i: load_fn!(loader, "glUniform2i", unsafe extern "system" fn(i32, i32, i32)),
                Uniform3i: load_fn!(loader, "glUniform3i", unsafe extern "system" fn(i32, i32, i32, i32)),
                Uniform4i: load_fn!(loader, "glUniform4i", unsafe extern "system" fn(i32, i32, i32, i32, i32)),
                Uniform1f: load_fn!(loader, "glUniform1f", unsafe extern "system" fn(i32, f32)),
                Uniform2f: load_fn!(loader, "glUniform2f", unsafe extern "system" fn(i32, f32, f32)),
                Uniform3f: load_fn!(loader, "glUniform3f", unsafe extern "system" fn(i32, f32, f32, f32)),
                Uniform4f: load_fn!(loader, "glUniform4f", unsafe extern "system" fn(i32, f32, f32, f32, f32)),
                Uniform1fv: load_fn!(loader, "glUniform1fv", unsafe extern "system" fn(i32, i32, *const f32)),
                Uniform3fv: load_fn!(loader, "glUniform3fv", unsafe extern "system" fn(i32, i32, *const f32)),
                Uniform4fv: load_fn!(loader, "glUniform4fv", unsafe extern "system" fn(i32, i32, *const f32)),
                UniformMatrix3fv: load_fn!(loader, "glUniformMatrix3fv", unsafe extern "system" fn(i32, i32, u8, *const f32)),
                UniformMatrix4fv: load_fn!(loader, "glUniformMatrix4fv", unsafe extern "system" fn(i32, i32, u8, *const f32)),

                GenTextures: load_fn!(loader, "glGenTextures", unsafe extern "system" fn(i32, *mut u32)),
                BindTexture: load_fn!(loader, "glBindTexture", unsafe extern "system" fn(u32, u32)),
                TexImage2D: load_fn!(loader, "glTexImage2D", unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void)),
                TexSubImage2D: load_fn!(loader, "glTexSubImage2D", unsafe extern "system" fn(u32, i32, i32, i32, i32, i32, u32, u32, *const c_void)),
                TexParameteri: load_fn!(loader, "glTexParameteri", unsafe extern "system" fn(u32, u32, i32)),
                TexParameterf: load_fn!(loader, "glTexParameterf", unsafe extern "system" fn(u32, u32, f32)),
                DeleteTextures: load_fn!(loader, "glDeleteTextures", unsafe extern "system" fn(i32, *const u32)),
                ActiveTexture: load_fn!(loader, "glActiveTexture", unsafe extern "system" fn(u32)),
                GenerateMipmap: load_fn!(loader, "glGenerateMipmap", unsafe extern "system" fn(u32), fallback: dummy_generate_mipmap),

                GenFramebuffers: load_fn!(loader, "glGenFramebuffers", unsafe extern "system" fn(i32, *mut u32)),
                BindFramebuffer: load_fn!(loader, "glBindFramebuffer", unsafe extern "system" fn(u32, u32)),
                DeleteFramebuffers: load_fn!(loader, "glDeleteFramebuffers", unsafe extern "system" fn(i32, *const u32)),
                FramebufferTexture2D: load_fn!(loader, "glFramebufferTexture2D", unsafe extern "system" fn(u32, u32, u32, u32, i32)),
                GenRenderbuffers: load_fn!(loader, "glGenRenderbuffers", unsafe extern "system" fn(i32, *mut u32)),
                BindRenderbuffer: load_fn!(loader, "glBindRenderbuffer", unsafe extern "system" fn(u32, u32)),
                RenderbufferStorage: load_fn!(loader, "glRenderbufferStorage", unsafe extern "system" fn(u32, u32, i32, i32)),
                FramebufferRenderbuffer: load_fn!(loader, "glFramebufferRenderbuffer", unsafe extern "system" fn(u32, u32, u32, u32)),
                DeleteRenderbuffers: load_fn!(loader, "glDeleteRenderbuffers", unsafe extern "system" fn(i32, *const u32)),
                CheckFramebufferStatus: load_fn!(loader, "glCheckFramebufferStatus", unsafe extern "system" fn(u32) -> u32),
                BlitFramebuffer: load_fn!(loader, "glBlitFramebuffer", unsafe extern "system" fn(i32, i32, i32, i32, i32, i32, i32, i32, u32, u32), fallback: dummy_blit_framebuffer),
                ReadPixels: load_fn!(loader, "glReadPixels", unsafe extern "system" fn(i32, i32, i32, i32, u32, u32, *mut c_void)),

                DrawArrays: load_fn!(loader, "glDrawArrays", unsafe extern "system" fn(u32, i32, i32)),
                DrawElements: load_fn!(loader, "glDrawElements", unsafe extern "system" fn(u32, i32, u32, *const c_void)),
                DrawArraysInstanced: load_fn!(loader, "glDrawArraysInstanced", unsafe extern "system" fn(u32, i32, i32, i32), fallback: dummy_draw_instanced),
                DrawElementsInstanced: load_fn!(loader, "glDrawElementsInstanced", unsafe extern "system" fn(u32, i32, u32, *const c_void, i32), fallback: dummy_draw_elements_instanced),
                DispatchCompute: load_fn!(loader, "glDispatchCompute", unsafe extern "system" fn(u32, u32, u32), fallback: dummy_dispatch_compute),
                MemoryBarrier: load_fn!(loader, "glMemoryBarrier", unsafe extern "system" fn(u32), fallback: dummy_memory_barrier),
            }
        }
    }
}
