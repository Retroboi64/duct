/**
 * Project32 - Duct Graphics Subsystem C-ABI Header
 * Compatible with C99, C11, C++, C#, Python ctypes, and FFI
 */

#ifndef DUCT_H
#define DUCT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32) || defined(__CYGWIN__)
    #ifdef DUCT_EXPORTS
        #define DUCT_API __declspec(dllexport)
    #else
        #define DUCT_API __declspec(dllimport)
    #endif
#else
    #if __GNUC__ >= 4
        #define DUCT_API __attribute__ ((visibility ("default")))
    #else
        #define DUCT_API
    #endif
#endif

/* Result status codes */
typedef enum DuctResult {
    DUCT_SUCCESS = 0,
    DUCT_ERROR_GENERIC = -1,
    DUCT_ERROR_INVALID_PARAMETER = -2,
    DUCT_ERROR_UNSUPPORTED = -3,
    DUCT_ERROR_INITIALIZATION_FAILED = -4,
    DUCT_ERROR_DEVICE_LOST = -5,
    DUCT_ERROR_RESOURCE_NOT_FOUND = -6,
    DUCT_ERROR_OUT_OF_MEMORY = -7,
    DUCT_ERROR_INVALID_STATE = -8,
    DUCT_ERROR_WINDOW_REQUIRED = -9
} DuctResult;

/* Backend API type */
typedef enum DuctBackendType {
    DUCT_BACKEND_UNKNOWN = 0,
    DUCT_BACKEND_OPENGL = 1,
    DUCT_BACKEND_DIRECTX12 = 2,
    DUCT_BACKEND_VULKAN = 3,
    DUCT_BACKEND_METAL = 4,
    DUCT_BACKEND_CUSTOM = 99
} DuctBackendType;

/* RGBA Color */
typedef struct DuctColor {
    float r;
    float g;
    float b;
    float a;
} DuctColor;

/* Native Window Handle descriptor */
typedef struct DuctWindowHandle {
    void* native_window;  /* E.g. HWND (Win32), Window XID (X11), NSView* (macOS) */
    void* native_display; /* E.g. HINSTANCE (Win32), Display* (X11) */
} DuctWindowHandle;

/* Platform-specific helper constructors */
static inline DuctWindowHandle duct_window_from_win32(void* hwnd, void* hinstance) {
    DuctWindowHandle h;
    h.native_window = hwnd;
    h.native_display = hinstance;
    return h;
}

static inline DuctWindowHandle duct_window_from_xlib(uint64_t window, void* display) {
    DuctWindowHandle h;
    h.native_window = (void*)(uintptr_t)window;
    h.native_display = display;
    return h;
}

static inline DuctWindowHandle duct_window_from_appkit(void* ns_view) {
    DuctWindowHandle h;
    h.native_window = ns_view;
    h.native_display = NULL;
    return h;
}

/* Initialization configuration */
typedef struct DuctInitConfig {
    DuctWindowHandle window;
    uint32_t width;
    uint32_t height;
    bool vsync;
    bool debug_mode;
    const char* app_name;
} DuctInitConfig;

/* Shader Stages */
typedef enum DuctShaderStage {
    DUCT_SHADER_STAGE_VERTEX = 1,
    DUCT_SHADER_STAGE_FRAGMENT = 2,
    DUCT_SHADER_STAGE_COMPUTE = 3
} DuctShaderStage;

/* Shader Descriptor */
typedef struct DuctShaderDesc {
    DuctShaderStage stage;
    const uint8_t* source_bytes;
    size_t source_len;
    const char* entry_point;
} DuctShaderDesc;

/* Resource Handles */
typedef uint64_t DuctShaderHandle;
typedef uint64_t DuctBufferHandle;
typedef uint64_t DuctTextureHandle;
typedef uint64_t DuctRenderTargetHandle;

/* Buffer Types */
typedef enum DuctBufferType {
    DUCT_BUFFER_TYPE_VERTEX = 1,
    DUCT_BUFFER_TYPE_INDEX = 2,
    DUCT_BUFFER_TYPE_UNIFORM = 3,
    DUCT_BUFFER_TYPE_STORAGE = 4
} DuctBufferType;

/* Buffer Descriptor */
typedef struct DuctBufferDesc {
    DuctBufferType buffer_type;
    size_t size_bytes;
    const uint8_t* initial_data;
    bool is_dynamic;
} DuctBufferDesc;

/* Texture Format */
typedef enum DuctTextureFormat {
    DUCT_TEXTURE_FORMAT_RGBA8 = 1,
    DUCT_TEXTURE_FORMAT_RGB8 = 2,
    DUCT_TEXTURE_FORMAT_R8 = 3
} DuctTextureFormat;

/* Texture Filter Mode */
typedef enum DuctFilterMode {
    DUCT_FILTER_NEAREST = 1,
    DUCT_FILTER_LINEAR = 2,
    DUCT_FILTER_LINEAR_MIPMAP_LINEAR = 3
} DuctFilterMode;

/* Texture Wrap Mode */
typedef enum DuctWrapMode {
    DUCT_WRAP_REPEAT = 1,
    DUCT_WRAP_CLAMP_TO_EDGE = 2,
    DUCT_WRAP_MIRRORED_REPEAT = 3
} DuctWrapMode;

/* Primitive Topology */
typedef enum DuctPrimitiveTopology {
    DUCT_TOPOLOGY_TRIANGLE_LIST = 1,
    DUCT_TOPOLOGY_TRIANGLE_STRIP = 2,
    DUCT_TOPOLOGY_LINE_LIST = 3,
    DUCT_TOPOLOGY_POINT_LIST = 4
} DuctPrimitiveTopology;

/* Comparison Operators */
typedef enum DuctCompareOp {
    DUCT_COMPARE_OP_ALWAYS = 0,
    DUCT_COMPARE_OP_NEVER = 1,
    DUCT_COMPARE_OP_LESS = 2,
    DUCT_COMPARE_OP_LESS_EQUAL = 3,
    DUCT_COMPARE_OP_EQUAL = 4,
    DUCT_COMPARE_OP_GREATER_EQUAL = 5,
    DUCT_COMPARE_OP_GREATER = 6,
    DUCT_COMPARE_OP_NOT_EQUAL = 7
} DuctCompareOp;

/* Index buffer format */
typedef enum DuctIndexFormat {
    DUCT_INDEX_FORMAT_U16 = 1,
    DUCT_INDEX_FORMAT_U32 = 2
} DuctIndexFormat;

/* Face Culling */
typedef enum DuctCullMode {
    DUCT_CULL_MODE_NONE = 0,
    DUCT_CULL_MODE_FRONT = 1,
    DUCT_CULL_MODE_BACK = 2
} DuctCullMode;

/* Blend Modes */
typedef enum DuctBlendMode {
    DUCT_BLEND_MODE_OPAQUE = 0,
    DUCT_BLEND_MODE_ALPHA_BLEND = 1,
    DUCT_BLEND_MODE_ADDITIVE = 2
} DuctBlendMode;

/* Vertex Format */
typedef enum DuctVertexFormat {
    DUCT_VERTEX_FORMAT_FLOAT = 1,
    DUCT_VERTEX_FORMAT_FLOAT2 = 2,
    DUCT_VERTEX_FORMAT_FLOAT3 = 3,
    DUCT_VERTEX_FORMAT_FLOAT4 = 4,
    DUCT_VERTEX_FORMAT_UBYTE4_NORMALIZED = 5
} DuctVertexFormat;

/* Vertex Attribute */
typedef struct DuctVertexAttribute {
    uint32_t location;
    DuctVertexFormat format;
    uint32_t offset;
} DuctVertexAttribute;

/* Vertex Layout Descriptor */
typedef struct DuctVertexLayout {
    DuctVertexAttribute attributes[8];
    uint32_t attribute_count;
    uint32_t stride;
} DuctVertexLayout;

/* Offscreen Render Target Descriptor */
typedef struct DuctRenderTargetDesc {
    uint32_t width;
    uint32_t height;
    DuctTextureFormat format;
    bool has_depth;
} DuctRenderTargetDesc;

/* Pipeline State Object Descriptor */
typedef struct DuctPipelineStateDesc {
    DuctShaderHandle vertex_shader;
    DuctShaderHandle fragment_shader;
    DuctVertexLayout vertex_layout;
    DuctCullMode cull_mode;
    DuctBlendMode blend_mode;
    bool depth_test;
    bool depth_write;
    DuctCompareOp depth_compare;
} DuctPipelineStateDesc;

/* Real-Time Frame Telemetry and GPU Performance Stats */
typedef struct DuctFrameStats {
    float fps;
    float frame_time_ms;
    uint64_t frame_count;
    uint32_t draw_calls;
    uint64_t triangles_rendered;
    uint32_t pipeline_switches;
    size_t allocated_gpu_memory_bytes;
} DuctFrameStats;

/* Raw Context Escape Hatch */
typedef struct DuctRawContext {
    DuctBackendType backend_type;
    void* context_handle;
    void* device_handle;
    const char* extra_info;
    void* (*get_proc_address)(const char* name);
} DuctRawContext;

/* Opaque Duct Controller Handle */
typedef void* DuctHandle;

/* ── Core Lifecycle Functions ────────────────────────────────────────── */

/**
 * Creates and initializes a Duct graphics controller.
 * NOTE: config->window.native_window must not be NULL (window required).
 */
DUCT_API DuctResult duct_create(const DuctInitConfig* config, DuctHandle* out_handle);

/**
 * Initializes Duct by inspecting command-line flags (--gl, --dx12, --vk).
 * Requires an attached native window handle.
 */
DUCT_API DuctResult duct_create_from_args(DuctWindowHandle window, DuctHandle* out_handle);

/**
 * Destroys a Duct controller, shutting down graphics pipelines and freeing resources.
 */
DUCT_API void duct_destroy(DuctHandle handle);

/**
 * Live switches the active graphics backend (1=OpenGL, 2=DirectX12, 3=Vulkan, 0=Auto)
 * at runtime on the same attached window without crashing.
 */
DUCT_API DuctResult duct_switch_backend(DuctHandle handle, uint32_t preference);

/* ── Surface & Frame Execution ───────────────────────────────────────── */

DUCT_API DuctResult duct_resize_viewport(DuctHandle handle, uint32_t width, uint32_t height);
DUCT_API DuctResult duct_set_scissor(DuctHandle handle, uint32_t x, uint32_t y, uint32_t width, uint32_t height);

DUCT_API DuctResult duct_begin_frame(DuctHandle handle, DuctColor clear_color);
DUCT_API DuctResult duct_end_frame(DuctHandle handle);
DUCT_API DuctResult duct_present(DuctHandle handle);

/* ── Resource Management ─────────────────────────────────────────────── */

DUCT_API DuctResult duct_create_shader(DuctHandle handle, const DuctShaderDesc* desc, DuctShaderHandle* out_shader);
DUCT_API DuctResult duct_reload_shader(DuctHandle handle, DuctShaderHandle shader, const DuctShaderDesc* desc);
DUCT_API DuctResult duct_destroy_shader(DuctHandle handle, DuctShaderHandle shader);

DUCT_API DuctResult duct_create_buffer(DuctHandle handle, const DuctBufferDesc* desc, DuctBufferHandle* out_buffer);
DUCT_API DuctResult duct_update_buffer(DuctHandle handle, DuctBufferHandle buffer, size_t offset, const void* data, size_t size);
DUCT_API DuctResult duct_destroy_buffer(DuctHandle handle, DuctBufferHandle buffer);
DUCT_API DuctResult duct_bind_vertex_buffer(DuctHandle handle, DuctBufferHandle buffer);
DUCT_API DuctResult duct_bind_index_buffer(DuctHandle handle, DuctBufferHandle buffer, DuctIndexFormat format);
DUCT_API DuctResult duct_bind_uniform_buffer(DuctHandle handle, uint32_t slot, DuctBufferHandle buffer);

DUCT_API DuctResult duct_create_texture_2d(DuctHandle handle, uint32_t width, uint32_t height, DuctTextureFormat format, const void* data, DuctTextureHandle* out_texture);
DUCT_API DuctResult duct_update_texture_2d(DuctHandle handle, DuctTextureHandle texture, uint32_t x, uint32_t y, uint32_t width, uint32_t height, const void* data, size_t size);
DUCT_API DuctResult duct_destroy_texture(DuctHandle handle, DuctTextureHandle texture);
DUCT_API DuctResult duct_bind_texture(DuctHandle handle, uint32_t slot, DuctTextureHandle texture);
DUCT_API DuctResult duct_generate_mipmaps(DuctHandle handle, DuctTextureHandle texture);
DUCT_API DuctResult duct_set_texture_filter(DuctHandle handle, DuctTextureHandle texture, DuctFilterMode min_filter, DuctFilterMode mag_filter);
DUCT_API DuctResult duct_set_texture_wrap(DuctHandle handle, DuctTextureHandle texture, DuctWrapMode wrap_s, DuctWrapMode wrap_t);

/* ── Render Targets (Offscreen Framebuffers) ─────────────────────────── */

DUCT_API DuctResult duct_create_render_target(DuctHandle handle, const DuctRenderTargetDesc* desc, DuctRenderTargetHandle* out_target);
DUCT_API DuctResult duct_destroy_render_target(DuctHandle handle, DuctRenderTargetHandle target);
DUCT_API DuctResult duct_bind_render_target(DuctHandle handle, DuctRenderTargetHandle target);
DUCT_API DuctResult duct_get_render_target_texture(DuctHandle handle, DuctRenderTargetHandle target, DuctTextureHandle* out_texture);
DUCT_API DuctResult duct_blit_render_target(DuctHandle handle, DuctRenderTargetHandle src, DuctRenderTargetHandle dst, uint32_t src_w, uint32_t src_h, uint32_t dst_w, uint32_t dst_h, DuctFilterMode filter);
DUCT_API DuctResult duct_read_pixels(DuctHandle handle, uint32_t x, uint32_t y, uint32_t width, uint32_t height, uint8_t* out_rgba, size_t buf_len);

/* ── Pipeline State & Drawing ────────────────────────────────────────── */

DUCT_API DuctResult duct_set_pipeline_state(DuctHandle handle, const DuctPipelineStateDesc* desc);
DUCT_API DuctResult duct_set_wireframe(DuctHandle handle, bool enabled);
DUCT_API DuctResult duct_set_vsync(DuctHandle handle, bool enabled);

DUCT_API DuctResult duct_set_uniform_int(DuctHandle handle, const char* name, int32_t val);
DUCT_API DuctResult duct_set_uniform_float(DuctHandle handle, const char* name, float val);
DUCT_API DuctResult duct_set_uniform_vec2(DuctHandle handle, const char* name, float x, float y);
DUCT_API DuctResult duct_set_uniform_vec3(DuctHandle handle, const char* name, float x, float y, float z);
DUCT_API DuctResult duct_set_uniform_vec4(DuctHandle handle, const char* name, float x, float y, float z, float w);
DUCT_API DuctResult duct_set_uniform_mat4(DuctHandle handle, const char* name, const float* elements_16);

DUCT_API DuctResult duct_dispatch_compute(DuctHandle handle, uint32_t groups_x, uint32_t groups_y, uint32_t groups_z);
DUCT_API DuctResult duct_draw(DuctHandle handle, uint32_t vertex_count, uint32_t first_vertex);
DUCT_API DuctResult duct_draw_indexed(DuctHandle handle, uint32_t index_count, uint32_t first_index);
DUCT_API DuctResult duct_draw_instanced(DuctHandle handle, uint32_t vertex_count, uint32_t instance_count, uint32_t first_vertex, uint32_t first_instance);
DUCT_API DuctResult duct_draw_indexed_instanced(DuctHandle handle, uint32_t index_count, uint32_t instance_count, uint32_t first_index, DuctIndexFormat index_format);

/* ── Telemetry & Escape Hatch ────────────────────────────────────────── */

DUCT_API DuctResult duct_get_frame_stats(DuctHandle handle, DuctFrameStats* out_stats);
DUCT_API const char* duct_get_backend_name(DuctHandle handle);
DUCT_API uint32_t duct_get_backend_type(DuctHandle handle);
DUCT_API DuctResult duct_get_raw_context(DuctHandle handle, DuctRawContext* out_raw);

#ifdef __cplusplus
}
#endif

#endif /* DUCT_H */
