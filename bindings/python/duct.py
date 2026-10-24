"""
Project32 Duct Graphics Subsystem - Python FFI Bindings
Uses standard Python ctypes (zero external dependencies).
Supports Windows, Linux, and macOS.
"""

import ctypes
import os
import sys

# Platform dynamic library names
if sys.platform == "win32":
    CANDIDATE_NAMES = ["duct.dll"]
elif sys.platform == "darwin":
    CANDIDATE_NAMES = ["libduct.dylib", "duct.dylib"]
else:
    CANDIDATE_NAMES = ["libduct.so", "duct.so"]

def _find_duct_dll():
    search_dirs = [
        os.path.abspath("."),
        os.path.abspath("./target/debug"),
        os.path.abspath("./target/release"),
        os.path.abspath("../../target/debug"),
        os.path.abspath("../../target/release"),
    ]
    for d in search_dirs:
        for name in CANDIDATE_NAMES:
            candidate = os.path.join(d, name)
            if os.path.isfile(candidate):
                return candidate
    return CANDIDATE_NAMES[0]

_lib = None

def get_lib():
    global _lib
    if _lib is None:
        path = _find_duct_dll()
        _lib = ctypes.CDLL(path)
        _setup_signatures(_lib)
    return _lib

# ── Types & Structs ──────────────────────────────────────────────────────────

class DuctColor(ctypes.Structure):
    _fields_ = [
        ("r", ctypes.c_float),
        ("g", ctypes.c_float),
        ("b", ctypes.c_float),
        ("a", ctypes.c_float),
    ]

    @classmethod
    def rgba(cls, r, g, b, a=1.0):
        return cls(r, g, b, a)

class DuctWindowHandle(ctypes.Structure):
    _fields_ = [
        ("native_window", ctypes.c_void_p),
        ("native_display", ctypes.c_void_p),
    ]

    @classmethod
    def from_hwnd(cls, hwnd):
        return cls(ctypes.c_void_p(hwnd), ctypes.c_void_p(0))

class DuctInitConfig(ctypes.Structure):
    _fields_ = [
        ("window", DuctWindowHandle),
        ("width", ctypes.c_uint32),
        ("height", ctypes.c_uint32),
        ("vsync", ctypes.c_bool),
        ("debug_mode", ctypes.c_bool),
        ("app_name", ctypes.c_char_p),
    ]

class DuctShaderDesc(ctypes.Structure):
    _fields_ = [
        ("stage", ctypes.c_uint32),
        ("source_bytes", ctypes.POINTER(ctypes.c_uint8)),
        ("source_len", ctypes.c_size_t),
        ("entry_point", ctypes.c_char_p),
    ]

class DuctBufferDesc(ctypes.Structure):
    _fields_ = [
        ("buffer_type", ctypes.c_uint32),
        ("size_bytes", ctypes.c_size_t),
        ("initial_data", ctypes.POINTER(ctypes.c_uint8)),
        ("is_dynamic", ctypes.c_bool),
    ]

class DuctVertexAttribute(ctypes.Structure):
    _fields_ = [
        ("location", ctypes.c_uint32),
        ("format", ctypes.c_uint32),
        ("offset", ctypes.c_uint32),
    ]

class DuctVertexLayout(ctypes.Structure):
    _fields_ = [
        ("attributes", DuctVertexAttribute * 8),
        ("attribute_count", ctypes.c_uint32),
        ("stride", ctypes.c_uint32),
    ]

class DuctRenderTargetDesc(ctypes.Structure):
    _fields_ = [
        ("width", ctypes.c_uint32),
        ("height", ctypes.c_uint32),
        ("format", ctypes.c_uint32),
        ("has_depth", ctypes.c_bool),
    ]

class DuctPipelineStateDesc(ctypes.Structure):
    _fields_ = [
        ("vertex_shader", ctypes.c_uint64),
        ("fragment_shader", ctypes.c_uint64),
        ("vertex_layout", DuctVertexLayout),
        ("cull_mode", ctypes.c_uint32),
        ("blend_mode", ctypes.c_uint32),
        ("depth_test", ctypes.c_bool),
        ("depth_write", ctypes.c_bool),
        ("depth_compare", ctypes.c_uint32),
    ]

class DuctFrameStats(ctypes.Structure):
    _fields_ = [
        ("fps", ctypes.c_float),
        ("frame_time_ms", ctypes.c_float),
        ("frame_count", ctypes.c_uint64),
        ("draw_calls", ctypes.c_uint32),
        ("triangles_rendered", ctypes.c_uint64),
        ("pipeline_switches", ctypes.c_uint32),
        ("allocated_gpu_memory_bytes", ctypes.c_size_t),
    ]

def _setup_signatures(lib):
    lib.duct_create.argtypes = [ctypes.POINTER(DuctInitConfig), ctypes.POINTER(ctypes.c_void_p)]
    lib.duct_create.restype = ctypes.c_int32

    lib.duct_create_from_args.argtypes = [DuctWindowHandle, ctypes.POINTER(ctypes.c_void_p)]
    lib.duct_create_from_args.restype = ctypes.c_int32

    lib.duct_destroy.argtypes = [ctypes.c_void_p]
    lib.duct_destroy.restype = None

    lib.duct_switch_backend.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
    lib.duct_switch_backend.restype = ctypes.c_int32

    lib.duct_resize_viewport.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_resize_viewport.restype = ctypes.c_int32

    lib.duct_begin_frame.argtypes = [ctypes.c_void_p, DuctColor]
    lib.duct_begin_frame.restype = ctypes.c_int32

    lib.duct_end_frame.argtypes = [ctypes.c_void_p]
    lib.duct_end_frame.restype = ctypes.c_int32

    lib.duct_present.argtypes = [ctypes.c_void_p]
    lib.duct_present.restype = ctypes.c_int32

    lib.duct_create_shader.argtypes = [ctypes.c_void_p, ctypes.POINTER(DuctShaderDesc), ctypes.POINTER(ctypes.c_uint64)]
    lib.duct_create_shader.restype = ctypes.c_int32

    lib.duct_reload_shader.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.POINTER(DuctShaderDesc)]
    lib.duct_reload_shader.restype = ctypes.c_int32

    lib.duct_destroy_shader.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
    lib.duct_destroy_shader.restype = ctypes.c_int32

    lib.duct_create_buffer.argtypes = [ctypes.c_void_p, ctypes.POINTER(DuctBufferDesc), ctypes.POINTER(ctypes.c_uint64)]
    lib.duct_create_buffer.restype = ctypes.c_int32

    lib.duct_destroy_buffer.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
    lib.duct_destroy_buffer.restype = ctypes.c_int32

    lib.duct_bind_vertex_buffer.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
    lib.duct_bind_vertex_buffer.restype = ctypes.c_int32

    lib.duct_bind_index_buffer.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.c_uint32]
    lib.duct_bind_index_buffer.restype = ctypes.c_int32

    lib.duct_bind_uniform_buffer.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint64]
    lib.duct_bind_uniform_buffer.restype = ctypes.c_int32

    lib.duct_create_texture_2d.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint64)]
    lib.duct_create_texture_2d.restype = ctypes.c_int32

    lib.duct_update_texture_2d.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_void_p, ctypes.c_size_t]
    lib.duct_update_texture_2d.restype = ctypes.c_int32

    lib.duct_destroy_texture.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
    lib.duct_destroy_texture.restype = ctypes.c_int32

    lib.duct_bind_texture.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint64]
    lib.duct_bind_texture.restype = ctypes.c_int32

    lib.duct_generate_mipmaps.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
    lib.duct_generate_mipmaps.restype = ctypes.c_int32

    lib.duct_set_texture_filter.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_set_texture_filter.restype = ctypes.c_int32

    lib.duct_set_texture_wrap.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_set_texture_wrap.restype = ctypes.c_int32

    lib.duct_create_render_target.argtypes = [ctypes.c_void_p, ctypes.POINTER(DuctRenderTargetDesc), ctypes.POINTER(ctypes.c_uint64)]
    lib.duct_create_render_target.restype = ctypes.c_int32

    lib.duct_destroy_render_target.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
    lib.duct_destroy_render_target.restype = ctypes.c_int32

    lib.duct_bind_render_target.argtypes = [ctypes.c_void_p, ctypes.c_uint64]
    lib.duct_bind_render_target.restype = ctypes.c_int32

    lib.duct_get_render_target_texture.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.POINTER(ctypes.c_uint64)]
    lib.duct_get_render_target_texture.restype = ctypes.c_int32

    lib.duct_blit_render_target.argtypes = [ctypes.c_void_p, ctypes.c_uint64, ctypes.c_uint64, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_blit_render_target.restype = ctypes.c_int32

    lib.duct_read_pixels.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t]
    lib.duct_read_pixels.restype = ctypes.c_int32

    lib.duct_set_pipeline_state.argtypes = [ctypes.c_void_p, ctypes.POINTER(DuctPipelineStateDesc)]
    lib.duct_set_pipeline_state.restype = ctypes.c_int32

    lib.duct_set_vsync.argtypes = [ctypes.c_void_p, ctypes.c_bool]
    lib.duct_set_vsync.restype = ctypes.c_int32

    lib.duct_set_uniform_int.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int32]
    lib.duct_set_uniform_int.restype = ctypes.c_int32

    lib.duct_dispatch_compute.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_dispatch_compute.restype = ctypes.c_int32

    lib.duct_draw.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_draw.restype = ctypes.c_int32

    lib.duct_draw_indexed.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_draw_indexed.restype = ctypes.c_int32

    lib.duct_draw_instanced.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_draw_instanced.restype = ctypes.c_int32

    lib.duct_draw_indexed_instanced.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_uint32]
    lib.duct_draw_indexed_instanced.restype = ctypes.c_int32

    lib.duct_draw_triangle.argtypes = [ctypes.c_void_p, ctypes.c_float, ctypes.c_float, ctypes.c_float, ctypes.c_float, ctypes.c_float, ctypes.c_float, DuctColor]
    lib.duct_draw_triangle.restype = ctypes.c_int32

    lib.duct_draw_rect.argtypes = [ctypes.c_void_p, ctypes.c_float, ctypes.c_float, ctypes.c_float, ctypes.c_float, DuctColor]
    lib.duct_draw_rect.restype = ctypes.c_int32

    lib.duct_draw_circle.argtypes = [ctypes.c_void_p, ctypes.c_float, ctypes.c_float, ctypes.c_float, ctypes.c_uint32, DuctColor]
    lib.duct_draw_circle.restype = ctypes.c_int32

    lib.duct_get_frame_stats.argtypes = [ctypes.c_void_p, ctypes.POINTER(DuctFrameStats)]
    lib.duct_get_frame_stats.restype = ctypes.c_int32

    lib.duct_get_backend_name.argtypes = [ctypes.c_void_p]
    lib.duct_get_backend_name.restype = ctypes.c_char_p

# ── High-level Python Wrapper ────────────────────────────────────────────────

class Duct:
    def __init__(self, hwnd, width=1280, height=720, vsync=True):
        self._lib = get_lib()
        self._handle = ctypes.c_void_p()
        
        cfg = DuctInitConfig()
        cfg.window.native_window = ctypes.c_void_p(hwnd)
        cfg.window.native_display = ctypes.c_void_p(0)
        cfg.width = width
        cfg.height = height
        cfg.vsync = vsync
        cfg.debug_mode = True
        cfg.app_name = b"Python Duct App"

        res = self._lib.duct_create(ctypes.byref(cfg), ctypes.byref(self._handle))
        if res != 0:
            raise RuntimeError(f"Failed to initialize Duct graphics pipeline (code {res})")

    @classmethod
    def from_args(cls, hwnd):
        obj = cls.__new__(cls)
        obj._lib = get_lib()
        obj._handle = ctypes.c_void_p()
        win = DuctWindowHandle.from_hwnd(hwnd)
        res = obj._lib.duct_create_from_args(win, ctypes.byref(obj._handle))
        if res != 0:
            raise RuntimeError(f"Failed to initialize Duct from CLI flags (code {res})")
        return obj

    def __del__(self):
        if hasattr(self, "_handle") and self._handle:
            self._lib.duct_destroy(self._handle)
            self._handle = None

    @property
    def backend_name(self):
        name = self._lib.duct_get_backend_name(self._handle)
        return name.decode("utf-8") if name else "Unknown"

    def switch_backend(self, preference):
        """Live switches backend: 1=OpenGL, 2=DirectX12, 3=Vulkan, 0=Auto"""
        res = self._lib.duct_switch_backend(self._handle, preference)
        if res != 0:
            raise RuntimeError(f"Failed to switch backend (code {res})")

    def resize_viewport(self, width, height):
        self._lib.duct_resize_viewport(self._handle, width, height)

    def begin_frame(self, r=0.1, g=0.2, b=0.3, a=1.0):
        color = DuctColor(r, g, b, a)
        self._lib.duct_begin_frame(self._handle, color)

    def end_frame(self):
        self._lib.duct_end_frame(self._handle)

    def present(self):
        self._lib.duct_present(self._handle)

    def create_shader(self, stage, source, entry="main"):
        src_bytes = source.encode("utf-8")
        raw_arr = (ctypes.c_uint8 * len(src_bytes)).from_buffer_copy(src_bytes)
        desc = DuctShaderDesc(
            stage=stage,
            source_bytes=raw_arr,
            source_len=len(src_bytes),
            entry_point=entry.encode("utf-8")
        )
        out_h = ctypes.c_uint64()
        res = self._lib.duct_create_shader(self._handle, ctypes.byref(desc), ctypes.byref(out_h))
        if res != 0:
            raise RuntimeError(f"Shader compilation failed (code {res})")
        return out_h.value

    def reload_shader(self, handle, stage, source, entry="main"):
        src_bytes = source.encode("utf-8")
        raw_arr = (ctypes.c_uint8 * len(src_bytes)).from_buffer_copy(src_bytes)
        desc = DuctShaderDesc(
            stage=stage,
            source_bytes=raw_arr,
            source_len=len(src_bytes),
            entry_point=entry.encode("utf-8")
        )
        res = self._lib.duct_reload_shader(self._handle, handle, ctypes.byref(desc))
        if res != 0:
            raise RuntimeError(f"Shader reload failed (code {res})")

    def create_buffer(self, buffer_type, byte_data, dynamic=False):
        raw_arr = (ctypes.c_uint8 * len(byte_data)).from_buffer_copy(byte_data)
        desc = DuctBufferDesc(
            buffer_type=buffer_type,
            size_bytes=len(byte_data),
            initial_data=raw_arr,
            is_dynamic=dynamic
        )
        out_h = ctypes.c_uint64()
        res = self._lib.duct_create_buffer(self._handle, ctypes.byref(desc), ctypes.byref(out_h))
        if res != 0:
            raise RuntimeError(f"Buffer creation failed (code {res})")
        return out_h.value

    def create_render_target(self, width, height, format=1, has_depth=True):
        desc = DuctRenderTargetDesc(width=width, height=height, format=format, has_depth=has_depth)
        out_h = ctypes.c_uint64()
        res = self._lib.duct_create_render_target(self._handle, ctypes.byref(desc), ctypes.byref(out_h))
        if res != 0:
            raise RuntimeError(f"Render target creation failed (code {res})")
        return out_h.value

    def bind_render_target(self, target_handle):
        self._lib.duct_bind_render_target(self._handle, target_handle)

    def get_render_target_texture(self, target_handle):
        out_h = ctypes.c_uint64()
        res = self._lib.duct_get_render_target_texture(self._handle, target_handle, ctypes.byref(out_h))
        if res != 0:
            raise RuntimeError(f"Failed to get render target texture (code {res})")
        return out_h.value

    def generate_mipmaps(self, texture_handle):
        self._lib.duct_generate_mipmaps(self._handle, texture_handle)

    def set_texture_filter(self, texture_handle, min_filter=2, mag_filter=2):
        self._lib.duct_set_texture_filter(self._handle, texture_handle, min_filter, mag_filter)

    def set_texture_wrap(self, texture_handle, wrap_s=1, wrap_t=1):
        self._lib.duct_set_texture_wrap(self._handle, texture_handle, wrap_s, wrap_t)

    def bind_vertex_buffer(self, buffer_handle):
        self._lib.duct_bind_vertex_buffer(self._handle, buffer_handle)

    def bind_index_buffer(self, buffer_handle, index_format=1):
        self._lib.duct_bind_index_buffer(self._handle, buffer_handle, index_format)

    def bind_uniform_buffer(self, slot, buffer_handle):
        self._lib.duct_bind_uniform_buffer(self._handle, slot, buffer_handle)

    def blit_render_target(self, src_rt, dst_rt, src_w, src_h, dst_w, dst_h, filter_mode=2):
        self._lib.duct_blit_render_target(self._handle, src_rt, dst_rt, src_w, src_h, dst_w, dst_h, filter_mode)

    def set_vsync(self, enabled):
        self._lib.duct_set_vsync(self._handle, enabled)

    def set_uniform_int(self, name, val):
        self._lib.duct_set_uniform_int(self._handle, name.encode("utf-8"), val)

    def dispatch_compute(self, groups_x, groups_y=1, groups_z=1):
        self._lib.duct_dispatch_compute(self._handle, groups_x, groups_y, groups_z)

    def set_pipeline_state(self, vs, fs, blend=0, cull=0, depth_test=True, depth_write=True, depth_compare=2):
        pso = DuctPipelineStateDesc()
        pso.vertex_shader = vs
        pso.fragment_shader = fs
        pso.cull_mode = cull
        pso.blend_mode = blend
        pso.depth_test = depth_test
        pso.depth_write = depth_write
        pso.depth_compare = depth_compare
        pso.vertex_layout.attribute_count = 0
        pso.vertex_layout.stride = 0
        self._lib.duct_set_pipeline_state(self._handle, ctypes.byref(pso))

    def draw(self, count, first=0):
        self._lib.duct_draw(self._handle, count, first)

    def draw_indexed(self, count, first=0):
        self._lib.duct_draw_indexed(self._handle, count, first)

    def draw_instanced(self, vertex_count, instance_count, first_vertex=0, first_instance=0):
        self._lib.duct_draw_instanced(self._handle, vertex_count, instance_count, first_vertex, first_instance)

    def draw_indexed_instanced(self, index_count, instance_count, first_index=0, index_format=1):
        self._lib.duct_draw_indexed_instanced(self._handle, index_count, instance_count, first_index, index_format)

    def draw_triangle(self, x0, y0, x1, y1, x2, y2, r=1.0, g=0.5, b=0.2, a=1.0):
        self._lib.duct_draw_triangle(self._handle, x0, y0, x1, y1, x2, y2, DuctColor(r, g, b, a))

    def draw_rect(self, x, y, width, height, r=0.2, g=0.8, b=0.4, a=1.0):
        self._lib.duct_draw_rect(self._handle, x, y, width, height, DuctColor(r, g, b, a))

    def draw_circle(self, center_x, center_y, radius, segments=32, r=0.3, g=0.6, b=1.0, a=1.0):
        self._lib.duct_draw_circle(self._handle, center_x, center_y, radius, segments, DuctColor(r, g, b, a))

    def get_frame_stats(self):
        stats = DuctFrameStats()
        res = self._lib.duct_get_frame_stats(self._handle, ctypes.byref(stats))
        if res != 0:
            raise RuntimeError(f"Failed to get frame stats (code {res})")
        return {
            "fps": stats.fps,
            "frame_time_ms": stats.frame_time_ms,
            "frame_count": stats.frame_count,
            "draw_calls": stats.draw_calls,
            "triangles_rendered": stats.triangles_rendered,
            "pipeline_switches": stats.pipeline_switches,
            "allocated_gpu_memory_bytes": stats.allocated_gpu_memory_bytes,
        }
