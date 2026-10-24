using System;
using System.Runtime.InteropServices;

namespace Project32.Duct
{
    public enum DuctResult : int
    {
        Success = 0,
        ErrorGeneric = -1,
        ErrorInvalidParameter = -2,
        ErrorUnsupported = -3,
        ErrorInitializationFailed = -4,
        ErrorDeviceLost = -5,
        ErrorResourceNotFound = -6,
        ErrorOutOfMemory = -7,
        ErrorInvalidState = -8,
        ErrorWindowRequired = -9
    }

    public enum DuctBackendType : uint
    {
        Unknown = 0,
        OpenGL = 1,
        DirectX12 = 2,
        Vulkan = 3,
        Metal = 4,
        DirectX11 = 5,
        Custom = 99
    }

    public enum DuctFilterMode : uint
    {
        Nearest = 1,
        Linear = 2,
        LinearMipmapLinear = 3
    }

    public enum DuctWrapMode : uint
    {
        Repeat = 1,
        ClampToEdge = 2,
        MirroredRepeat = 3
    }

    public enum DuctCompareOp : uint
    {
        Always = 0,
        Never = 1,
        Less = 2,
        LessEqual = 3,
        Equal = 4,
        GreaterEqual = 5,
        Greater = 6,
        NotEqual = 7
    }

    public enum DuctIndexFormat : uint
    {
        U16 = 1,
        U32 = 2
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctColor
    {
        public float R, G, B, A;

        public DuctColor(float r, float g, float b, float a = 1.0f)
        {
            R = r; G = g; B = b; A = a;
        }

        public static DuctColor CornflowerBlue => new DuctColor(0.392f, 0.584f, 0.929f, 1.0f);
        public static DuctColor Black => new DuctColor(0.0f, 0.0f, 0.0f, 1.0f);
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctWindowHandle
    {
        public IntPtr NativeWindow;
        public IntPtr NativeDisplay;

        public DuctWindowHandle(IntPtr window, IntPtr display = default)
        {
            NativeWindow = window;
            NativeDisplay = display;
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctInitConfig
    {
        public DuctWindowHandle Window;
        public uint Width;
        public uint Height;
        [MarshalAs(UnmanagedType.I1)] public bool VSync;
        [MarshalAs(UnmanagedType.I1)] public bool DebugMode;
        [MarshalAs(UnmanagedType.LPStr)] public string AppName;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctShaderDesc
    {
        public uint Stage;
        public IntPtr SourceBytes;
        public UIntPtr SourceLen;
        [MarshalAs(UnmanagedType.LPStr)] public string EntryPoint;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctBufferDesc
    {
        public uint BufferType;
        public UIntPtr SizeBytes;
        public IntPtr InitialData;
        [MarshalAs(UnmanagedType.I1)] public bool IsDynamic;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctVertexAttribute
    {
        public uint Location;
        public uint Format;
        public uint Offset;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctVertexLayout
    {
        [MarshalAs(UnmanagedType.ByValArray, SizeConst = 8)]
        public DuctVertexAttribute[] Attributes;
        public uint AttributeCount;
        public uint Stride;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctRenderTargetDesc
    {
        public uint Width;
        public uint Height;
        public uint Format;
        [MarshalAs(UnmanagedType.I1)] public bool HasDepth;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctPipelineStateDesc
    {
        public ulong VertexShader;
        public ulong FragmentShader;
        public DuctVertexLayout VertexLayout;
        public uint CullMode;
        public uint BlendMode;
        [MarshalAs(UnmanagedType.I1)] public bool DepthTest;
        [MarshalAs(UnmanagedType.I1)] public bool DepthWrite;
        public uint DepthCompare;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct DuctFrameStats
    {
        public float Fps;
        public float FrameTimeMs;
        public ulong FrameCount;
        public uint DrawCalls;
        public ulong TrianglesRendered;
        public uint PipelineSwitches;
        public UIntPtr AllocatedGpuMemoryBytes;
    }

    public static class DuctNative
    {
        private const string LibName = "duct";

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_create(ref DuctInitConfig config, out IntPtr outHandle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_create_from_args(DuctWindowHandle window, out IntPtr outHandle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void duct_destroy(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_switch_backend(IntPtr handle, uint preference);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_resize_viewport(IntPtr handle, uint width, uint height);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_begin_frame(IntPtr handle, DuctColor clearColor);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_end_frame(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_present(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_create_shader(IntPtr handle, ref DuctShaderDesc desc, out ulong outShader);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_reload_shader(IntPtr handle, ulong shader, ref DuctShaderDesc desc);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_destroy_shader(IntPtr handle, ulong shader);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_create_buffer(IntPtr handle, ref DuctBufferDesc desc, out ulong outBuffer);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_destroy_buffer(IntPtr handle, ulong buffer);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_bind_vertex_buffer(IntPtr handle, ulong buffer);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_bind_index_buffer(IntPtr handle, ulong buffer, DuctIndexFormat format);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_bind_uniform_buffer(IntPtr handle, uint slot, ulong buffer);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_create_render_target(IntPtr handle, ref DuctRenderTargetDesc desc, out ulong outTarget);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_destroy_render_target(IntPtr handle, ulong target);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_bind_render_target(IntPtr handle, ulong target);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_get_render_target_texture(IntPtr handle, ulong target, out ulong outTexture);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_blit_render_target(IntPtr handle, ulong src, ulong dst, uint srcW, uint srcH, uint dstW, uint dstH, DuctFilterMode filter);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_read_pixels(IntPtr handle, uint x, uint y, uint width, uint height, IntPtr outRgba, UIntPtr bufLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_generate_mipmaps(IntPtr handle, ulong texture);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_set_texture_filter(IntPtr handle, ulong texture, DuctFilterMode minFilter, DuctFilterMode magFilter);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_set_texture_wrap(IntPtr handle, ulong texture, DuctWrapMode wrapS, DuctWrapMode wrapT);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_set_pipeline_state(IntPtr handle, ref DuctPipelineStateDesc desc);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_set_vsync(IntPtr handle, [MarshalAs(UnmanagedType.I1)] bool enabled);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_set_uniform_int(IntPtr handle, [MarshalAs(UnmanagedType.LPStr)] string name, int val);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_dispatch_compute(IntPtr handle, uint groupsX, uint groupsY, uint groupsZ);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_draw(IntPtr handle, uint vertexCount, uint firstVertex);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_draw_indexed(IntPtr handle, uint indexCount, uint firstIndex);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_draw_instanced(IntPtr handle, uint vertexCount, uint instanceCount, uint firstVertex, uint firstInstance);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_draw_indexed_instanced(IntPtr handle, uint indexCount, uint instanceCount, uint firstIndex, DuctIndexFormat indexFormat);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_draw_triangle(IntPtr handle, float x0, float y0, float x1, float y1, float x2, float y2, DuctColor color);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_draw_rect(IntPtr handle, float x, float y, float width, float height, DuctColor color);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_draw_circle(IntPtr handle, float centerX, float centerY, float radius, uint segments, DuctColor color);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern DuctResult duct_get_frame_stats(IntPtr handle, out DuctFrameStats outStats);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr duct_get_backend_name(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern uint duct_get_backend_type(IntPtr handle);
    }
}
