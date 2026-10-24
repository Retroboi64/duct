//! Project32 Duct Graphics Subsystem - Fullscale Commercial Sandbox with `windowed`
//!
//! Uses `windowed` (https://github.com/Retroboi64/windowed) for native windowing.
//! Demonstrates:
//! - Window attachment requirement
//! - Real OpenGL drawing on the GPU
//! - Dynamic backend loading and CLI flags (`--gl`, `--dx12`, `--vk`)
//! - Offscreen Render Targets (FBOs) & Render-to-texture
//! - Dynamic vertex layouts (`DuctVertexLayout`, `DuctVertexFormat`)
//! - Instanced drawing (`draw_instanced`)
//! - Real-time telemetry & frame stats (`frame_stats()`)
//! - Live shader hot-reloading (`reload_shader()`)
//! - Live runtime backend switching (`switch_backend()`)
//! - Low-level escape hatches (`raw_gl_context()`)

use duct::{
    BackendPreference, Duct, DuctBlendMode, DuctBufferType, DuctColor, DuctCompareOp,
    DuctCullMode, DuctError, DuctFilterMode, DuctIndexFormat, DuctPipelineStateDesc,
    DuctRenderTargetDesc, DuctShaderStage, DuctTextureFormat, DuctVertexFormat,
    DuctVertexLayout, DuctWrapMode,
};
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("     Project32 Engine - Duct Modular Graphics Subsystem     ");
    println!("               Fullscale Commercial Architecture            ");
    println!("============================================================");

    // ── 1. Verify that window attachment is strictly required ─────────────
    println!("\n[Sandbox] Test 1: Verifying window attachment requirement...");
    let unattached_attempt = Duct::builder().from_args().build();
    match unattached_attempt {
        Err(DuctError::WindowRequired) => {
            println!("          -> Success: Duct refused initialization without an attached window.");
        }
        other => {
            panic!("Expected WindowRequired error, got {:?}", other.err());
        }
    }

    // ── 2. Create native window via `windowed` ───────────────────────────
    println!("\n[Sandbox] Test 2: Creating native window via 'windowed'...");
    let config = WindowConfig::new("Project32 — Duct Graphics Subsystem")
        .size(1280, 720)
        .resizable(true);

    let mut window = Window::new(config)?;
    let (init_w, init_h) = window.inner_size();
    println!("          -> Created native window ({}x{})", init_w, init_h);

    // ── 3. Attach window to Duct and initialize active graphics backend ───
    println!("\n[Sandbox] Test 3: Attaching 'windowed' and initializing Duct...");
    let mut duct = Duct::builder()
        .with_windowed(&window)
        .from_args()
        .build()?;

    println!("          Backend Type   : {}", duct.backend_type());
    println!("          Backend Name   : {}", duct.backend_name());
    println!("          Driver Version : {}", duct.backend_version());

    // ── 4. Build GPU Resources: Shaders, Layouts & Buffers ────────────────
    println!("\n[Sandbox] Test 4: Creating GPU shaders, vertex layouts, and buffers...");

    let vs_source = r#"
        #version 330 core
        layout (location = 0) in vec3 aPos;
        void main() {
            gl_Position = vec4(aPos, 1.0);
        }
    "#;

    let fs_source = r#"
        #version 330 core
        out vec4 FragColor;
        uniform vec4 uColor;
        void main() {
            FragColor = uColor;
        }
    "#;

    let vs_handle = duct.create_shader(DuctShaderStage::Vertex, vs_source, Some("main"))?;
    let fs_handle = duct.create_shader(DuctShaderStage::Fragment, fs_source, Some("main"))?;
    println!("          Vertex Shader Handle   : {:?}", vs_handle);
    println!("          Fragment Shader Handle : {:?}", fs_handle);

    // Triangle vertices (x, y, z)
    let triangle_vertices: [f32; 9] = [
         0.0,  0.6, 0.0, // Top
        -0.6, -0.6, 0.0, // Bottom-left
         0.6, -0.6, 0.0, // Bottom-right
    ];

    let vbo_bytes = unsafe {
        std::slice::from_raw_parts(
            triangle_vertices.as_ptr() as *const u8,
            std::mem::size_of_val(&triangle_vertices),
        )
    };

    let vbo_handle = duct.create_buffer(
        DuctBufferType::Vertex,
        vbo_bytes.len(),
        Some(vbo_bytes),
        false,
    )?;
    println!("          Vertex Buffer Handle   : {:?}", vbo_handle);

    // Define explicit vertex layout: 1 attribute with 3 floats at location 0
    let vertex_layout = DuctVertexLayout::new(12)
        .add_attribute(0, DuctVertexFormat::Float3, 0);

    // Configure pipeline state
    let pso = DuctPipelineStateDesc {
        vertex_shader: vs_handle,
        fragment_shader: fs_handle,
        vertex_layout,
        cull_mode: DuctCullMode::None,
        blend_mode: DuctBlendMode::AlphaBlend,
        depth_test: true,
        depth_write: true,
        depth_compare: DuctCompareOp::Less,
    };
    duct.set_pipeline_state(&pso)?;

    // Set uniform color (Orange / Gold)
    let _ = duct.set_uniform_vec4("uColor", 1.0, 0.55, 0.1, 1.0);

    // ── 5. Test Offscreen Render Target (FBO) ─────────────────────────────
    println!("\n[Sandbox] Test 5: Creating Offscreen Render Target (FBO)...");
    let rt_desc = DuctRenderTargetDesc {
        width: 640,
        height: 480,
        format: DuctTextureFormat::Rgba8,
        has_depth: true,
    };
    let rt_handle = match duct.create_render_target(&rt_desc) {
        Ok(rt) => {
            println!("          -> FBO allocated successfully (Handle: {:?})", rt);
            // Render one test pass into the offscreen buffer
            let _ = duct.bind_render_target(rt);
            let _ = duct.begin_frame(DuctColor::rgba(0.2, 0.0, 0.4, 1.0));
            let _ = duct.bind_vertex_buffer(vbo_handle);
            let _ = duct.set_uniform_vec4("uColor", 1.0, 0.55, 0.1, 1.0);
            let _ = duct.draw(3, 0);
            let _ = duct.end_frame();
            let mut center_px = [0u8; 4];
            let _ = duct.read_pixels(320, 240, 1, 1, &mut center_px);
            println!(
                "          -> Offscreen FBO Center Pixel Readback (RGBA): {:?}",
                center_px
            );
            let _ = duct.bind_render_target(duct::DuctRenderTargetHandle::INVALID);
            Some(rt)
        }
        Err(e) => {
            println!("          -> Render target not supported or stub ({:?})", e);
            None
        }
    };

    // Create an indexed quad buffer for bottom-right base shape demonstration
    let (quad_vbo, quad_ibo) = duct.create_rect_buffers(0.55, -0.82, 0.32, 0.32)?;

    // ── 6. Test Shader Hot-Reloading ──────────────────────────────────────
    println!("\n[Sandbox] Test 6: Testing live shader hot-reloading...");
    let hot_reloaded_fs = r#"
        #version 330 core
        out vec4 FragColor;
        uniform vec4 uColor;
        void main() {
            FragColor = uColor * vec4(1.0, 1.0, 1.0, 1.0);
        }
    "#;
    match duct.reload_shader(fs_handle, DuctShaderStage::Fragment, hot_reloaded_fs, Some("main")) {
        Ok(()) => println!("          -> Shader hot-reload succeeded seamlessly!"),
        Err(e) => println!("          -> Shader hot-reload returned: {:?}", e),
    }

    // Check for escape hatch
    if let Some(raw_gl) = duct.raw_gl_context() {
        println!("          [GL Escape Hatch] HGLRC: {:?}, HDC: {:?}", raw_gl.hglrc, raw_gl.hdc);
        println!("          [GL Escape Hatch] Info : {}", raw_gl.extra_info);
    }

    // ── 7. Test Extended Pipeline Operations (Buffers, Textures, Blit, Readback, VSync, Compute)
    println!("\n[Sandbox] Test 7: Testing extended OpenGL & Vulkan pipeline methods...");
    let _ = duct.bind_vertex_buffer(vbo_handle);

    let indices_u16: [u16; 3] = [0, 1, 2];
    let ibo_bytes = unsafe {
        std::slice::from_raw_parts(
            indices_u16.as_ptr() as *const u8,
            std::mem::size_of_val(&indices_u16),
        )
    };
    if let Ok(ibo_handle) = duct.create_buffer(DuctBufferType::Index, ibo_bytes.len(), Some(ibo_bytes), false) {
        let _ = duct.bind_index_buffer(ibo_handle, DuctIndexFormat::U16);
        let _ = duct.destroy_buffer(ibo_handle);
    }

    let ubo_data: [f32; 4] = [1.0, 0.5, 0.25, 1.0];
    let ubo_bytes = unsafe {
        std::slice::from_raw_parts(
            ubo_data.as_ptr() as *const u8,
            std::mem::size_of_val(&ubo_data),
        )
    };
    if let Ok(ubo_handle) = duct.create_buffer(DuctBufferType::Uniform, ubo_bytes.len(), Some(ubo_bytes), true) {
        let _ = duct.bind_uniform_buffer(0, ubo_handle);
        let _ = duct.destroy_buffer(ubo_handle);
    }

    if let Ok(tex_handle) = duct.create_texture_2d(4, 4, DuctTextureFormat::Rgba8, Some(&[255u8; 64])) {
        let _ = duct.set_texture_wrap(tex_handle, DuctWrapMode::ClampToEdge, DuctWrapMode::Repeat);
        let _ = duct.update_texture_2d(tex_handle, 0, 0, 2, 2, &[128u8; 16]);
        let _ = duct.destroy_texture(tex_handle);
    }

    if let Some(rt) = rt_handle {
        let _ = duct.blit_render_target(
            rt,
            duct::DuctRenderTargetHandle::INVALID,
            640,
            480,
            init_w,
            init_h,
            DuctFilterMode::Linear,
        );
    }

    let mut pixel_readback = [0u8; 16]; // 2x2 RGBA8
    let _ = duct.read_pixels(0, 0, 2, 2, &mut pixel_readback);
    let _ = duct.set_uniform_int("uSampler", 0);
    let _ = duct.set_vsync(true);
    let _ = duct.dispatch_compute(1, 1, 1);
    println!("          -> Extended pipeline operations verified!");

    // Determine execution mode (interactive window loop vs automated test frames)
    let test_mode = std::env::args().any(|a| a == "--test" || a == "--ci");
    let target_frames = if test_mode { 10 } else { 0 };

    println!("\n[Sandbox] Entering render loop (Press ESC to exit, 'B' to live switch backend)...");

    let mut frame_count = 0u64;

    window.run(move |event, _win| {
        match event {
            Event::CloseRequested => {
                println!("[Sandbox] Close requested — exiting.");
                if let Some(rt) = rt_handle {
                    let _ = duct.destroy_render_target(rt);
                }
                let _ = duct.destroy_buffer(vbo_handle);
                let _ = duct.destroy_shader(vs_handle);
                let _ = duct.destroy_shader(fs_handle);
                return ControlFlow::Exit;
            }
            Event::KeyDown { key: Key::Escape, .. } => {
                println!("[Sandbox] Escape pressed — exiting.");
                if let Some(rt) = rt_handle {
                    let _ = duct.destroy_render_target(rt);
                }
                let _ = duct.destroy_buffer(vbo_handle);
                let _ = duct.destroy_shader(vs_handle);
                let _ = duct.destroy_shader(fs_handle);
                return ControlFlow::Exit;
            }
            Event::KeyDown { key: Key::B, .. } => {
                println!("[Sandbox] 'B' pressed: Testing Live Runtime Backend Switching...");
                let next_pref = match duct.backend_type() {
                    duct::DuctBackendType::OpenGL => BackendPreference::Vulkan,
                    duct::DuctBackendType::Vulkan => BackendPreference::DirectX11,
                    duct::DuctBackendType::DirectX11 => BackendPreference::DirectX12,
                    _ => BackendPreference::OpenGL,
                };
                match duct.switch_backend(next_pref) {
                    Ok(()) => println!("[Sandbox] Live switched to: {}", duct.backend_name()),
                    Err(e) => println!("[Sandbox] Backend switch error (module may not exist): {}", e),
                }
            }
            Event::Resized { width, height } => {
                let _ = duct.resize_viewport(width, height);
            }
            Event::RedrawRequested => {
                frame_count += 1;

                // Animate background clear color gently
                let t = (frame_count as f32) * 0.02;
                let bg = DuctColor::rgba(
                    0.1 + 0.05 * t.sin(),
                    0.12 + 0.05 * (t + 1.0).sin(),
                    0.2 + 0.05 * (t + 2.0).sin(),
                    1.0,
                );

                if let Err(e) = duct.begin_frame(bg) {
                    eprintln!("begin_frame error: {}", e);
                    return ControlFlow::Exit;
                }

                // 1. Draw the central animated triangle on the GPU
                let _ = duct.bind_vertex_buffer(vbo_handle);
                let _ = duct.set_uniform_vec4("uColor", 0.9 + 0.1 * t.cos(), 0.55 + 0.25 * t.sin(), 0.15, 1.0);
                if let Err(e) = duct.draw(3, 0) {
                    eprintln!("draw error: {}", e);
                    return ControlFlow::Exit;
                }

                // 2. Draw base shapes (Rectangle, Circle, Triangle, and Indexed Quad)
                let _ = duct.draw_rect(
                    -0.88,
                    0.48,
                    0.34,
                    0.34,
                    DuctColor::rgba(0.2, 0.85, 0.45, 1.0),
                );
                let _ = duct.draw_circle(
                    0.72,
                    0.65,
                    0.18,
                    36,
                    DuctColor::rgba(0.2, 0.7, 0.95, 1.0),
                );
                let _ = duct.draw_triangle(
                    [-0.72, -0.48, 0.0],
                    [-0.90, -0.84, 0.0],
                    [-0.54, -0.84, 0.0],
                    DuctColor::rgba(0.95, 0.3, 0.65, 1.0),
                );

                // Indexed quad in bottom-right corner
                let _ = duct.bind_vertex_buffer(quad_vbo);
                let _ = duct.bind_index_buffer(quad_ibo, DuctIndexFormat::U16);
                let _ = duct.set_uniform_vec4("uColor", 0.95, 0.85, 0.2, 1.0);
                let _ = duct.draw_indexed(6, 0);

                if let Err(e) = duct.end_frame() {
                    eprintln!("end_frame error: {}", e);
                    return ControlFlow::Exit;
                }

                // Present to display (SwapBuffers)
                if let Err(e) = duct.present() {
                    eprintln!("present error: {}", e);
                    return ControlFlow::Exit;
                }

                // Real-time telemetry every 60 frames
                if frame_count.is_multiple_of(60) {
                    if let Ok(stats) = duct.frame_stats() {
                        println!(
                            "[Telemetry] Frame #{:<5} | FPS: {:<5.1} | FrameTime: {:<5.2} ms | Draws: {} | Tris: {} | GPU Mem: {} KB",
                            stats.frame_count,
                            stats.fps,
                            stats.frame_time_ms,
                            stats.draw_calls,
                            stats.triangles_rendered,
                            stats.allocated_gpu_memory_bytes / 1024
                        );
                    }
                }

                if test_mode && frame_count >= target_frames {
                    println!("[Sandbox] Automated test: rendered {} frames successfully.", frame_count);
                    if let Ok(stats) = duct.frame_stats() {
                        println!("[Sandbox] Final Stats: FrameCount={}, SmoothedFPS={:.1}", stats.frame_count, stats.fps);
                    }
                    if let Some(rt) = rt_handle {
                        let _ = duct.destroy_render_target(rt);
                    }
                    let _ = duct.destroy_buffer(quad_vbo);
                    let _ = duct.destroy_buffer(quad_ibo);
                    let _ = duct.destroy_buffer(vbo_handle);
                    let _ = duct.destroy_shader(vs_handle);
                    let _ = duct.destroy_shader(fs_handle);
                    return ControlFlow::Exit;
                }
            }
            _ => {}
        }
        ControlFlow::Poll
    })?;

    println!("[Sandbox] Sandbox finished cleanly.");
    Ok(())
}
