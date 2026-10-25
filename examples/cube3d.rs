//! 3D Rotating Textured Cube Example for Project32 Duct.
//! Demonstrates:
//! - Full 3D perspective projection and LookAt camera
//! - Model-View-Projection (MVP) matrix uploading via `set_uniform_mat4`
//! - Depth testing (`depth_test: true, depth_compare: Less`)
//! - Indexed rendering (`draw_indexed`) of a 24-vertex 36-index 3D mesh
//! - Live backend hot-switching (`B` key) across OpenGL, Vulkan, DX11, and DX12

use duct::{
    BackendPreference, Duct, DuctBlendMode, DuctBufferType, DuctColor, DuctCompareOp,
    DuctCullMode, DuctIndexFormat, DuctPipelineStateDesc, DuctShaderStage, DuctVertexFormat,
    DuctVertexLayout, Mat4, Vec3,
};
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("           Project32 Engine - Duct 3D Cube Example          ");
    println!("============================================================");

    let mut window = Window::new(
        WindowConfig::new("Project32 Duct — 3D Cube")
            .size(1280, 720)
            .resizable(true),
    )?;

    let mut duct = Duct::builder()
        .with_windowed(&window)
        .from_args()
        .vsync(true)
        .build()?;

    println!("[Cube3D] Active Backend: {}", duct.backend_name());

    // ── 1. Create Shaders ─────────────────────────────────────────────────
    let vs_source = r#"
        #version 330 core
        layout (location = 0) in vec3 aPos;
        layout (location = 1) in vec3 aColor;
        layout (location = 2) in vec2 aUv;

        uniform mat4 uMVP;

        out vec3 vColor;
        out vec2 vUv;

        void main() {
            gl_Position = uMVP * vec4(aPos, 1.0);
            vColor = aColor;
            vUv = aUv;
        }
    "#;

    let fs_source = r#"
        #version 330 core
        in vec3 vColor;
        in vec2 vUv;
        out vec4 FragColor;

        void main() {
            // High-contrast procedural checkerboard pattern on each cube face
            vec2 check = step(0.5, fract(vUv * 4.0));
            float c = (check.x == check.y) ? 1.0 : 0.65;
            FragColor = vec4(vColor * c, 1.0);
        }
    "#;

    let vs = duct.create_shader(DuctShaderStage::Vertex, vs_source, Some("main"))?;
    let fs = duct.create_shader(DuctShaderStage::Fragment, fs_source, Some("main"))?;

    // ── 2. Create 3D Cube Geometry (24 vertices: pos(3), color(3), uv(2)) ─
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

    // ── 3. Configure 3D Pipeline State with Depth Buffer ──────────────────
    let vertex_layout = DuctVertexLayout::new(32)
        .add_attribute(0, DuctVertexFormat::Float3, 0)   // aPos:   offset 0
        .add_attribute(1, DuctVertexFormat::Float3, 12)  // aColor: offset 12
        .add_attribute(2, DuctVertexFormat::Float2, 24); // aUv:    offset 24

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

    let test_mode = std::env::args().any(|a| a == "--test" || a == "--ci");
    let target_frames = if test_mode { 10 } else { 0 };

    println!("[Cube3D] Entering 3D render loop (Press ESC to exit, 'B' to switch backend)...");

    let mut frame_count = 0u64;
    let (mut width, mut height) = window.inner_size();

    window.run(move |event, _win| {
        match event {
            Event::CloseRequested | Event::KeyDown { key: Key::Escape, .. } => {
                println!("[Cube3D] Exiting cleanly.");
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
                    eprintln!("[Cube3D] Backend switch error: {}", e);
                } else {
                    println!("[Cube3D] Switched to: {}", duct.backend_name());
                    // Re-apply pipeline state on new backend
                    let _ = duct.set_pipeline_state(&pso);
                }
            }
            Event::Resized { width: w, height: h } => {
                width = w;
                height = h;
                let _ = duct.resize_viewport(w, h);
            }
            Event::RedrawRequested => {
                frame_count += 1;
                let t = (frame_count as f32) * 0.02;

                // 3D Matrix Calculations: MVP
                let aspect = (width as f32) / (height.max(1) as f32);
                let proj = Mat4::perspective(45.0f32.to_radians(), aspect, 0.1, 100.0);
                let view = Mat4::look_at(
                    Vec3::new(3.0 * t.sin(), 2.2, 3.0 * t.cos()),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::UP,
                );
                let model = Mat4::rotation_y(t * 0.7) * Mat4::rotation_x(t * 0.4);
                let mvp = proj * view * model;

                // Begin frame with dark slate background
                let _ = duct.begin_frame(DuctColor::rgba(0.08, 0.09, 0.12, 1.0));

                // Upload uniform transformation matrix
                let _ = duct.set_uniform_mat4("uMVP", &mvp.to_cols_array());

                // Bind geometry and draw 3D cube
                let _ = duct.bind_vertex_buffer(vbo);
                let _ = duct.bind_index_buffer(ibo, DuctIndexFormat::U16);
                let _ = duct.draw_indexed(36, 0);

                let _ = duct.end_frame();
                let _ = duct.present();

                if frame_count % 60 == 0 {
                    if let Ok(stats) = duct.frame_stats() {
                        println!("[Cube3D] Frame #{:<5} | FPS: {:<5.1} | DrawCalls: {} | Triangles: {}", stats.frame_count, stats.fps, stats.draw_calls, stats.triangles_rendered);
                    }
                }

                if test_mode && frame_count >= target_frames {
                    println!("[Cube3D] Automated test complete (rendered {} frames).", frame_count);
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
