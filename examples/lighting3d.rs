//! 3D Lighting & Shading Example for Project32 Duct.
//! Demonstrates:
//! - Full 3D Blinn-Phong illumination model (Ambient, Diffuse, Specular)
//! - Directional sunlight and an animated orbiting 3D point light
//! - Procedurally generated 3D UV Sphere with per-vertex normal vectors
//! - Dynamic normal matrix transformations and camera-space specular highlights
//! - Live backend hot-switching (`B` key) across OpenGL, Vulkan, DX11, and DX12

use duct::{
    BackendPreference, Duct, DuctBlendMode, DuctBufferType, DuctColor, DuctCompareOp,
    DuctCullMode, DuctIndexFormat, DuctPipelineStateDesc, DuctShaderStage, DuctVertexFormat,
    DuctVertexLayout, Mat4, Vec3,
};
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("        Project32 Engine - Duct 3D Lighting Example         ");
    println!("      Real-Time Phong / Blinn-Phong Directional & Point     ");
    println!("============================================================");

    let mut window = Window::new(
        WindowConfig::new("Project32 Duct — 3D Lighting")
            .size(1280, 720)
            .resizable(true),
    )?;

    let mut duct = Duct::builder()
        .with_windowed(&window)
        .from_args()
        .vsync(true)
        .build()?;

    println!("[Lighting3D] Active Backend: {}", duct.backend_name());

    // ── 1. Create Phong Lighting Shaders ──────────────────────────────────
    let vs_source = r#"
        #version 330 core
        layout (location = 0) in vec3 aPos;
        layout (location = 1) in vec3 aNormal;
        layout (location = 2) in vec2 aUv;

        uniform mat4 uMVP;
        uniform mat4 uModel;

        out vec3 vFragPos;
        out vec3 vNormal;
        out vec2 vUv;

        void main() {
            vec4 worldPos = uModel * vec4(aPos, 1.0);
            vFragPos = worldPos.xyz;

            // In our rotation-only model matrix, the 3x3 model matrix acts as normal matrix
            vNormal = mat3(uModel) * aNormal;
            vUv = aUv;

            gl_Position = uMVP * vec4(aPos, 1.0);
        }
    "#;

    let fs_source = r#"
        #version 330 core
        in vec3 vFragPos;
        in vec3 vNormal;
        in vec2 vUv;

        uniform vec3 uViewPos;
        uniform vec3 uPointLightPos;
        uniform vec3 uPointLightColor;
        uniform vec3 uDirLightDir;
        uniform vec3 uDirLightColor;
        uniform vec4 uObjectColor;

        out vec4 FragColor;

        void main() {
            vec3 norm = normalize(vNormal);
            vec3 viewDir = normalize(uViewPos - vFragPos);

            // 1. Ambient
            vec3 ambient = 0.15 * vec3(1.0, 1.0, 1.0);

            // 2. Directional Sunlight
            vec3 sunDir = normalize(-uDirLightDir);
            float sunDiff = max(dot(norm, sunDir), 0.0);
            vec3 sunHalf = normalize(sunDir + viewDir);
            float sunSpec = pow(max(dot(norm, sunHalf), 0.0), 32.0);
            vec3 sunLight = (sunDiff * 0.7 + sunSpec * 0.4) * uDirLightColor;

            // 3. Orbiting Point Light with Distance Attenuation
            vec3 pointDir = normalize(uPointLightPos - vFragPos);
            float dist = length(uPointLightPos - vFragPos);
            float atten = 1.0 / (1.0 + 0.14 * dist + 0.07 * (dist * dist));

            float pointDiff = max(dot(norm, pointDir), 0.0);
            vec3 pointHalf = normalize(pointDir + viewDir);
            float pointSpec = pow(max(dot(norm, pointHalf), 0.0), 64.0);
            vec3 pointLight = (pointDiff * 1.0 + pointSpec * 0.8) * uPointLightColor * atten;

            // Material pattern
            vec2 check = step(0.5, fract(vUv * 8.0));
            float pat = (check.x == check.y) ? 1.0 : 0.85;

            vec3 totalLight = (ambient + sunLight + pointLight) * (uObjectColor.rgb * pat);
            FragColor = vec4(totalLight, 1.0);
        }
    "#;

    let vs = duct.create_shader(DuctShaderStage::Vertex, vs_source, Some("main"))?;
    let fs = duct.create_shader(DuctShaderStage::Fragment, fs_source, Some("main"))?;

    // ── 2. Procedural 3D UV Sphere (Positions, Normals, UVs) ──────────────
    let (vertices, indices) = generate_uv_sphere(1.2, 24, 32);
    let vbo_bytes = unsafe {
        std::slice::from_raw_parts(vertices.as_ptr() as *const u8, vertices.len() * std::mem::size_of::<f32>())
    };
    let vbo = duct.create_buffer(DuctBufferType::Vertex, vbo_bytes.len(), Some(vbo_bytes), false)?;

    let ibo_bytes = unsafe {
        std::slice::from_raw_parts(indices.as_ptr() as *const u8, indices.len() * std::mem::size_of::<u16>())
    };
    let ibo = duct.create_buffer(DuctBufferType::Index, ibo_bytes.len(), Some(ibo_bytes), false)?;

    // ── 3. Pipeline State with Depth Test ─────────────────────────────────
    let vertex_layout = DuctVertexLayout::new(32)
        .add_attribute(0, DuctVertexFormat::Float3, 0)   // aPos:    offset 0
        .add_attribute(1, DuctVertexFormat::Float3, 12)  // aNormal: offset 12
        .add_attribute(2, DuctVertexFormat::Float2, 24); // aUv:     offset 24

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

    println!("[Lighting3D] Entering lighting render loop (Press ESC to exit, 'B' to switch backend)...");

    let mut frame_count = 0u64;
    let (mut width, mut height) = window.inner_size();

    window.run(move |event, _win| {
        match event {
            Event::CloseRequested | Event::KeyDown { key: Key::Escape, .. } => {
                println!("[Lighting3D] Exiting cleanly.");
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
                    eprintln!("[Lighting3D] Backend switch error: {}", e);
                } else {
                    println!("[Lighting3D] Switched to: {}", duct.backend_name());
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
                let t = (frame_count as f32) * 0.025;

                // Orbiting Point Light Position (rotating in 3D around sphere)
                let light_x = 2.4 * t.cos();
                let light_y = 1.0 + 0.8 * (t * 1.5).sin();
                let light_z = 2.4 * t.sin();

                // Camera Position
                let cam_pos = Vec3::new(0.0, 1.8, 4.2);
                let aspect = (width as f32) / (height.max(1) as f32);
                let proj = Mat4::perspective(45.0f32.to_radians(), aspect, 0.1, 50.0);
                let view = Mat4::look_at(cam_pos, Vec3::new(0.0, 0.0, 0.0), Vec3::UP);
                let model = Mat4::rotation_y(t * 0.5) * Mat4::rotation_x(t * 0.3);
                let mvp = proj * view * model;

                let _ = duct.begin_frame(DuctColor::rgba(0.05, 0.06, 0.09, 1.0));

                // Upload uniform transformation matrices and light parameters
                let _ = duct.set_uniform_mat4("uMVP", &mvp.to_cols_array());
                let _ = duct.set_uniform_mat4("uModel", &model.to_cols_array());

                let _ = duct.set_uniform_vec3("uViewPos", cam_pos.x, cam_pos.y, cam_pos.z);
                let _ = duct.set_uniform_vec3("uPointLightPos", light_x, light_y, light_z);
                let _ = duct.set_uniform_vec3("uPointLightColor", 1.0, 0.65, 0.2); // Warm Golden light
                let _ = duct.set_uniform_vec3("uDirLightDir", -0.5, -1.0, -0.5);
                let _ = duct.set_uniform_vec3("uDirLightColor", 0.4, 0.6, 0.9);   // Cool Sky light
                let _ = duct.set_uniform_vec4("uObjectColor", 0.2, 0.75, 0.85, 1.0); // Cyan/Teal material

                // Draw Lit 3D Sphere
                let _ = duct.bind_vertex_buffer(vbo);
                let _ = duct.bind_index_buffer(ibo, DuctIndexFormat::U16);
                let _ = duct.draw_indexed(indices.len() as u32, 0);

                let _ = duct.end_frame();
                let _ = duct.present();

                if frame_count % 60 == 0 {
                    if let Ok(stats) = duct.frame_stats() {
                        println!("[Lighting3D] Frame #{:<5} | FPS: {:<5.1} | Triangles: {}", stats.frame_count, stats.fps, stats.triangles_rendered);
                    }
                }

                if test_mode && frame_count >= target_frames {
                    println!("[Lighting3D] Automated test complete (rendered {} frames).", frame_count);
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

/// Generates an indexed UV Sphere with position(3), normal(3), and uv(2) per vertex.
fn generate_uv_sphere(radius: f32, rings: usize, sectors: usize) -> (Vec<f32>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let r_step = std::f32::consts::PI / (rings as f32);
    let s_step = std::f32::consts::TAU / (sectors as f32);

    for r in 0..=rings {
        let phi = std::f32::consts::FRAC_PI_2 - (r as f32) * r_step;
        let y = phi.sin();
        let cos_phi = phi.cos();

        for s in 0..=sectors {
            let theta = (s as f32) * s_step;
            let x = cos_phi * theta.cos();
            let z = cos_phi * theta.sin();

            let u = (s as f32) / (sectors as f32);
            let v = (r as f32) / (rings as f32);

            // Position (x, y, z)
            vertices.push(x * radius);
            vertices.push(y * radius);
            vertices.push(z * radius);

            // Normal (unit direction on sphere)
            vertices.push(x);
            vertices.push(y);
            vertices.push(z);

            // UV coordinates
            vertices.push(u);
            vertices.push(v);
        }
    }

    for r in 0..rings {
        let k1 = (r * (sectors + 1)) as u16;
        let k2 = (k1 + sectors as u16 + 1) as u16;

        for s in 0..sectors {
            let s16 = s as u16;
            if r != 0 {
                indices.push(k1 + s16);
                indices.push(k2 + s16);
                indices.push(k1 + s16 + 1);
            }
            if r != (rings - 1) {
                indices.push(k1 + s16 + 1);
                indices.push(k2 + s16);
                indices.push(k2 + s16 + 1);
            }
        }
    }

    (vertices, indices)
}
