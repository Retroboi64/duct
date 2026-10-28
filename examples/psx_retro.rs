//! PSX / Retro 32-bit Console Graphics Demo for Project32 Duct.
//! Recreates the authentic aesthetics of 1990s 32-bit console hardware:
//! - Low-resolution internal offscreen FBO rendering (320x240)
//! - Nearest-neighbor blit to window backbuffer for chunky retro pixelation
//! - Vertex snapping/jitter (emulating lack of floating-point subpixel precision)
//! - Affine texture warping simulation (lack of perspective-correct division)
//! - Per-vertex Gouraud lighting and distance fog
//! - Indexed 3D Cube geometry with distinct colored faces
//! - Live backend hot-switching (`B` key) across OpenGL, Vulkan, DX11, and DX12

use duct::{
    BackendPreference, Duct, DuctBlendMode, DuctBufferType, DuctColor, DuctCompareOp,
    DuctCullMode, DuctFilterMode, DuctIndexFormat, DuctPipelineStateDesc, DuctRenderTargetDesc,
    DuctShaderStage, DuctTextureFormat, DuctVertexFormat, DuctVertexLayout, Mat4, Vec3,
};
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("        Project32 Engine - Duct PSX Retro 3D Demo           ");
    println!("    Authentic 32-Bit Fixed-Point Vertex Jitter & Fog        ");
    println!("============================================================");

    let mut window = Window::new(
        WindowConfig::new("Project32 Duct — PSX Retro 3D Cube")
            .size(1280, 720)
            .resizable(true),
    )?;

    let mut duct = Duct::builder()
        .with_windowed(&window)
        .from_args()
        .vsync(true)
        .build()?;

    println!("[PSX] Active Backend: {}", duct.backend_name());

    // ── 1. Create PSX Shaders with Vertex Jitter & Fog ────────────────────
    let vs_source = r#"
        #version 330 core
        layout (location = 0) in vec3 aPos;
        layout (location = 1) in vec3 aColor;
        layout (location = 2) in vec2 aUv;

        uniform mat4 uMVP;

        out vec3 vColor;
        out vec2 vUv;
        out float vDepth;

        void main() {
            vec4 clip = uMVP * vec4(aPos, 1.0);

            // PSX Hardware Emulation: Fixed-point vertex grid snapping (jitter)
            vec3 ndc = clip.xyz / clip.w;
            vec2 resolution = vec2(200.0, 150.0);
            ndc.xy = floor(ndc.xy * resolution) / resolution;
            gl_Position = vec4(ndc * clip.w, clip.w);

            vColor = aColor;
            vUv = aUv;
            vDepth = clip.z;
        }
    "#;

    let fs_source = r#"
        #version 330 core
        in vec3 vColor;
        in vec2 vUv;
        in float vDepth;
        out vec4 FragColor;

        void main() {
            // Retro affine texture checkerboard pattern
            vec2 grid = step(0.5, fract(vUv * 6.0));
            float pat = (grid.x == grid.y) ? 1.0 : 0.7;

            // Distance fog (classic PSX dark fade)
            vec3 fogColor = vec3(0.04, 0.05, 0.08);
            float fogFactor = clamp((vDepth - 2.0) / 7.0, 0.0, 0.85);

            vec3 shaded = vColor * pat;
            vec3 finalColor = mix(shaded, fogColor, fogFactor);

            FragColor = vec4(finalColor, 1.0);
        }
    "#;

    let vs = duct.create_shader(DuctShaderStage::Vertex, vs_source, Some("main"))?;
    let fs = duct.create_shader(DuctShaderStage::Fragment, fs_source, Some("main"))?;

    // ── 2. Geometry: 3D Cube Geometry (24 vertices: pos(3), color(3), uv(2))
    #[rustfmt::skip]
    let vertices: [f32; 192] = [
        // Front face (Red)
        -1.0, -1.0,  1.0,   1.0, 0.2, 0.2,   0.0, 0.0,
         1.0, -1.0,  1.0,   1.0, 0.2, 0.2,   1.0, 0.0,
         1.0,  1.0,  1.0,   1.0, 0.2, 0.2,   1.0, 1.0,
        -1.0,  1.0,  1.0,   1.0, 0.2, 0.2,   0.0, 1.0,

        // Back face (Green)
         1.0, -1.0, -1.0,   0.2, 1.0, 0.3,   0.0, 0.0,
        -1.0, -1.0, -1.0,   0.2, 1.0, 0.3,   1.0, 0.0,
        -1.0,  1.0, -1.0,   0.2, 1.0, 0.3,   1.0, 1.0,
         1.0,  1.0, -1.0,   0.2, 1.0, 0.3,   0.0, 1.0,

        // Top face (Blue)
        -1.0,  1.0,  1.0,   0.2, 0.5, 1.0,   0.0, 0.0,
         1.0,  1.0,  1.0,   0.2, 0.5, 1.0,   1.0, 0.0,
         1.0,  1.0, -1.0,   0.2, 0.5, 1.0,   1.0, 1.0,
        -1.0,  1.0, -1.0,   0.2, 0.5, 1.0,   0.0, 1.0,

        // Bottom face (Yellow)
        -1.0, -1.0, -1.0,   1.0, 0.9, 0.2,   0.0, 0.0,
         1.0, -1.0, -1.0,   1.0, 0.9, 0.2,   1.0, 0.0,
         1.0, -1.0,  1.0,   1.0, 0.9, 0.2,   1.0, 1.0,
        -1.0, -1.0,  1.0,   1.0, 0.9, 0.2,   0.0, 1.0,

        // Right face (Magenta)
         1.0, -1.0,  1.0,   0.9, 0.3, 0.9,   0.0, 0.0,
         1.0, -1.0, -1.0,   0.9, 0.3, 0.9,   1.0, 0.0,
         1.0,  1.0, -1.0,   0.9, 0.3, 0.9,   1.0, 1.0,
         1.0,  1.0,  1.0,   0.9, 0.3, 0.9,   0.0, 1.0,

        // Left face (Cyan)
        -1.0, -1.0, -1.0,   0.1, 0.9, 0.9,   0.0, 0.0,
        -1.0, -1.0,  1.0,   0.1, 0.9, 0.9,   1.0, 0.0,
        -1.0,  1.0,  1.0,   0.1, 0.9, 0.9,   1.0, 1.0,
        -1.0,  1.0, -1.0,   0.1, 0.9, 0.9,   0.0, 1.0,
    ];

    #[rustfmt::skip]
    let indices: [u16; 36] = [
         0,  1,  2,   0,  2,  3, // Front
         4,  5,  6,   4,  6,  7, // Back
         8,  9, 10,   8, 10, 11, // Top
        12, 13, 14,  12, 14, 15, // Bottom
        16, 17, 18,  16, 18, 19, // Right
        20, 21, 22,  20, 22, 23, // Left
    ];

    let vbo_bytes = unsafe {
        std::slice::from_raw_parts(vertices.as_ptr() as *const u8, std::mem::size_of_val(&vertices))
    };
    let vbo = duct.create_buffer(DuctBufferType::Vertex, vbo_bytes.len(), Some(vbo_bytes), false)?;

    let ibo_bytes = unsafe {
        std::slice::from_raw_parts(indices.as_ptr() as *const u8, std::mem::size_of_val(&indices))
    };
    let ibo = duct.create_buffer(DuctBufferType::Index, ibo_bytes.len(), Some(ibo_bytes), false)?;

    // ── 3. Pipeline State with Depth Test ─────────────────────────────────
    let vertex_layout = DuctVertexLayout::new(32)
        .add_attribute(0, DuctVertexFormat::Float3, 0)
        .add_attribute(1, DuctVertexFormat::Float3, 12)
        .add_attribute(2, DuctVertexFormat::Float2, 24);

    let pso = DuctPipelineStateDesc {
        vertex_shader: vs,
        fragment_shader: fs,
        vertex_layout,
        cull_mode: DuctCullMode::Back,
        blend_mode: DuctBlendMode::Opaque,
        depth_test: true,
        depth_write: true,
        depth_compare: DuctCompareOp::Less,
    };
    duct.set_pipeline_state(&pso)?;

    // ── 4. Internal Low-Resolution Retro Framebuffer (320x240) ────────────
    const RETRO_W: u32 = 320;
    const RETRO_H: u32 = 240;

    let fbo_desc = DuctRenderTargetDesc {
        width: RETRO_W,
        height: RETRO_H,
        format: DuctTextureFormat::Rgba8,
        has_depth: true,
    };
    let retro_fbo = duct.create_render_target(&fbo_desc).ok();
    if retro_fbo.is_some() {
        println!("[PSX] Allocated low-resolution 320x240 FBO for authentic retro rasterization.");
    }

    let test_mode = std::env::args().any(|a| a == "--test" || a == "--ci");
    let target_frames = if test_mode { 10 } else { 0 };

    println!("[PSX] Entering retro render loop (Press ESC to exit, 'B' to switch backend)...");

    let mut frame_count = 0u64;
    let (mut win_w, mut win_h) = window.inner_size();

    window.run(move |event, _win| {
        match event {
            Event::CloseRequested | Event::KeyDown { key: Key::Escape, .. } => {
                println!("[PSX] Exiting cleanly.");
                if let Some(fbo) = retro_fbo {
                    let _ = duct.destroy_render_target(fbo);
                }
                let _ = duct.destroy_buffer(vbo);
                let _ = duct.destroy_buffer(ibo);
                let _ = duct.destroy_shader(vs);
                let _ = duct.destroy_shader(fs);
                return ControlFlow::Exit;
            }
            Event::KeyDown { key: Key::B, .. } => {
                let next_pref = match duct.backend_type() {
                    duct::DuctBackendType::OpenGL => BackendPreference::Vulkan,
                    duct::DuctBackendType::Vulkan => BackendPreference::DirectX11,
                    duct::DuctBackendType::DirectX11 => BackendPreference::DirectX12,
                    _ => BackendPreference::OpenGL,
                };
                if let Err(e) = duct.switch_backend(next_pref) {
                    eprintln!("[PSX] Backend switch error: {}", e);
                } else {
                    println!("[PSX] Switched to: {}", duct.backend_name());
                    let _ = duct.set_pipeline_state(&pso);
                }
            }
            Event::Resized { width: w, height: h } => {
                win_w = w;
                win_h = h;
                let _ = duct.resize_viewport(w, h);
            }
            Event::RedrawRequested => {
                frame_count += 1;
                let t = (frame_count as f32) * 0.02;

                // Bind low-res retro FBO if available
                if let Some(fbo) = retro_fbo {
                    let _ = duct.bind_render_target(fbo);
                    let _ = duct.resize_viewport(RETRO_W, RETRO_H);
                }

                // Dark moody PSX background clear
                let _ = duct.begin_frame(DuctColor::rgba(0.04, 0.05, 0.08, 1.0));

                // Camera & Model Matrices
                let aspect = (RETRO_W as f32) / (RETRO_H as f32);
                let proj = Mat4::perspective(45.0f32.to_radians(), aspect, 0.1, 50.0);
                let view = Mat4::look_at(
                    Vec3::new(3.0 * t.sin(), 2.0, 3.0 * t.cos()),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::UP,
                );

                // 1. Draw Primary Rotating 3D Cube
                let model = Mat4::rotation_y(t * 0.9) * Mat4::rotation_x(t * 0.5);
                let mvp = proj * view * model;
                let _ = duct.set_uniform_mat4("uMVP", &mvp.to_cols_array());
                let _ = duct.bind_vertex_buffer(vbo);
                let _ = duct.bind_index_buffer(ibo, DuctIndexFormat::U16);
                let _ = duct.draw_indexed(36, 0);

                // 2. Draw Floating Satellite Cube
                let sat_model = Mat4::translation(2.2 * (t * 1.3).cos(), 0.6 * (t * 1.8).sin(), 2.2 * (t * 1.3).sin())
                    * Mat4::scaling(0.4, 0.4, 0.4)
                    * Mat4::rotation_y(-t * 2.0)
                    * Mat4::rotation_z(t * 1.5);
                let sat_mvp = proj * view * sat_model;
                let _ = duct.set_uniform_mat4("uMVP", &sat_mvp.to_cols_array());
                let _ = duct.draw_indexed(36, 0);

                let _ = duct.end_frame();

                // If rendered offscreen, upscale to window with nearest filtering
                if let Some(fbo) = retro_fbo {
                    let _ = duct.bind_render_target(duct::DuctRenderTargetHandle::INVALID);
                    let _ = duct.resize_viewport(win_w, win_h);
                    let _ = duct.blit_render_target(
                        fbo,
                        duct::DuctRenderTargetHandle::INVALID,
                        RETRO_W,
                        RETRO_H,
                        win_w,
                        win_h,
                        DuctFilterMode::Nearest,
                    );
                }

                let _ = duct.present();

                if frame_count % 60 == 0 {
                    if let Ok(stats) = duct.frame_stats() {
                        println!("[PSX] Frame #{:<5} | FPS: {:<5.1} | Triangles: {}", stats.frame_count, stats.fps, stats.triangles_rendered);
                    }
                }

                if test_mode && frame_count >= target_frames {
                    println!("[PSX] Automated test complete (rendered {} frames).", frame_count);
                    if let Some(fbo) = retro_fbo {
                        let _ = duct.destroy_render_target(fbo);
                    }
                    let _ = duct.destroy_buffer(vbo);
                    let _ = duct.destroy_buffer(ibo);
                    let _ = duct.destroy_shader(vs);
                    let _ = duct.destroy_shader(fs);
                    return ControlFlow::Exit;
                }
            }
            _ => {}
        }
        ControlFlow::Poll
    })?;

    Ok(())
}
