//! Project32 Duct Graphics Subsystem - Advanced Shader Playground
//!
//! Demonstrates:
//! - Custom Vertex & Fragment Shaders
//! - Uniform buffer & dynamic uniform uploads (`set_uniform_float`, `set_uniform_vec2`, `set_uniform_int`)
//! - Procedural generation:
//!     1. Cyberpunk Neon Plasma / Warp
//!     2. Retro CRT Monitor (Barrel Distortion, Scanlines, Chromatic Aberration)
//!     3. Real-Time 3D Raymarched Gyroid / SDF Minimal Surface
//!     4. Water Ripple & Underwater Caustics
//! - Live Shader Hot-Reloading (`reload_shader`)
//! - Multi-backend runtime switching (OpenGL, Vulkan, DirectX 11, DirectX 12)
//!
//! Controls:
//! - [1, 2, 3, 4]  : Switch active shader procedural effect
//! - [R]           : Live hot-reload shader from source
//! - [B]           : Switch graphics backend (GL -> VK -> DX11 -> DX12)
//! - [Space]       : Pause / Resume animation time
//! - [ESC]         : Exit

use duct::{
    BackendPreference, Duct, DuctBlendMode, DuctBufferType, DuctColor, DuctCompareOp,
    DuctCullMode, DuctIndexFormat, DuctPipelineStateDesc, DuctShaderStage,
    DuctVertexFormat, DuctVertexLayout,
};
use windowed::{ControlFlow, Event, Key, Window, WindowConfig};

const VS_SHADER_GLSL: &str = r#"#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;

out vec2 vUV;

void main() {
    vUV = aTexCoord;
    gl_Position = vec4(aPos, 1.0);
}
"#;

const FS_SHADER_GLSL: &str = r#"#version 330 core
out vec4 FragColor;

in vec2 vUV;

uniform float u_time;
uniform vec2  u_resolution;
uniform vec2  u_mouse;
uniform int   u_mode;

// ── Mode 1: Cyberpunk Neon Plasma ─────────────────────────────────────────
vec4 renderPlasma(vec2 uv, float t) {
    vec2 p = uv * 2.0 - 1.0;
    p.x *= u_resolution.x / max(u_resolution.y, 1.0);

    float v1 = sin(p.x * 6.0 + t);
    float v2 = sin(p.y * 6.0 + t * 0.7);
    float v3 = sin((p.x + p.y) * 5.0 + t * 1.3);
    float dist = length(p);
    float v4 = sin(dist * 8.0 - t * 2.0);

    float val = v1 + v2 + v3 + v4;

    vec3 col1 = vec3(0.05, 0.95, 0.90); // Cyan neon
    vec3 col2 = vec3(0.95, 0.05, 0.65); // Magenta neon
    vec3 col3 = vec3(0.40, 0.10, 0.90); // Deep Violet

    vec3 color = mix(col1, col2, sin(val * 1.5) * 0.5 + 0.5);
    color = mix(color, col3, cos(dist * 4.0 - t) * 0.5 + 0.5);

    // Glowing core pulse
    color += vec3(0.15) / (dist + 0.3);

    return vec4(color, 1.0);
}

// ── Mode 2: Retro CRT Monitor & Chromatic Aberration ───────────────────────
vec2 crtDistortion(vec2 uv) {
    vec2 p = uv * 2.0 - 1.0;
    vec2 offset = p.yx / 5.0;
    p += p * offset * offset;
    return p * 0.5 + 0.5;
}

vec4 renderCRT(vec2 uv, float t) {
    vec2 crt_uv = crtDistortion(uv);

    // Dark border outside monitor curvature
    if (crt_uv.x < 0.0 || crt_uv.x > 1.0 || crt_uv.y < 0.0 || crt_uv.y > 1.0) {
        return vec4(0.03, 0.03, 0.05, 1.0);
    }

    // Chromatic aberration (RGB channel split)
    float split = 0.006 * sin(t * 1.5 + crt_uv.y * 10.0);
    vec2 uv_r = crt_uv + vec2(split, 0.0);
    vec2 uv_b = crt_uv - vec2(split, 0.0);

    // Base background grid
    vec2 grid = fract(crt_uv * vec2(32.0, 18.0));
    float grid_line = step(0.92, grid.x) + step(0.92, grid.y);

    float r = sin(uv_r.x * 12.0 + t) * 0.5 + 0.5 + grid_line * 0.4;
    float g = cos(crt_uv.y * 10.0 - t) * 0.5 + 0.5 + grid_line * 0.4;
    float b = sin((uv_b.x + uv_b.y) * 8.0 + t * 2.0) * 0.5 + 0.5 + grid_line * 0.4;

    vec3 color = vec3(r * 0.2, g * 0.9, b * 0.4); // Phosphor green tint

    // Scanlines
    float scanline = sin(crt_uv.y * u_resolution.y * 1.5) * 0.25;
    color -= scanline;

    // Vignette
    float vig = crt_uv.x * crt_uv.y * (1.0 - crt_uv.x) * (1.0 - crt_uv.y);
    vig = clamp(pow(16.0 * vig, 0.35), 0.0, 1.0);
    color *= vig;

    // Flicker
    color *= 0.96 + 0.04 * sin(t * 60.0);

    return vec4(color, 1.0);
}

// ── Mode 3: Raymarched 3D Gyroid / Minimal Surface ────────────────────────
float mapGyroid(vec3 p, float t) {
    vec3 q = p * 1.8;
    float g = dot(sin(q), cos(q.zxy)) + 0.15 * sin(t * 2.0);
    float sphere = length(p) - 2.8;
    return max(abs(g) - 0.15, sphere);
}

vec3 calcNormal(vec3 p, float t) {
    float h = 0.001;
    vec2 k = vec2(1.0, -1.0);
    return normalize(
        k.xyy * mapGyroid(p + k.xyy * h, t) +
        k.yyx * mapGyroid(p + k.yyx * h, t) +
        k.yxy * mapGyroid(p + k.yxy * h, t) +
        k.xxx * mapGyroid(p + k.xxx * h, t)
    );
}

vec4 renderRaymarchedGyroid(vec2 uv, float t) {
    vec2 p = (uv * 2.0 - 1.0);
    p.x *= u_resolution.x / max(u_resolution.y, 1.0);

    // Camera setup
    float cam_dist = 4.0;
    float rot = t * 0.4;
    vec3 ro = vec3(sin(rot) * cam_dist, 1.2 * cos(rot * 0.5), cos(rot) * cam_dist);
    vec3 ta = vec3(0.0, 0.0, 0.0);
    vec3 ww = normalize(ta - ro);
    vec3 uu = normalize(cross(ww, vec3(0.0, 1.0, 0.0)));
    vec3 vv = cross(uu, ww);
    vec3 rd = normalize(p.x * uu + p.y * vv + 1.8 * ww);

    // Raymarching loop
    float d = 0.0;
    float t_dist = 0.0;
    bool hit = false;
    vec3 pos = ro;

    for (int i = 0; i < 64; ++i) {
        pos = ro + rd * t_dist;
        d = mapGyroid(pos, t);
        if (abs(d) < 0.002) {
            hit = true;
            break;
        }
        t_dist += d * 0.7;
        if (t_dist > 10.0) break;
    }

    if (hit) {
        vec3 n = calcNormal(pos, t);
        vec3 light_dir = normalize(vec3(0.6, 0.8, -0.5));
        float diff = max(dot(n, light_dir), 0.0);
        float spec = pow(max(dot(reflect(-light_dir, n), -rd), 0.0), 32.0);

        // Iridescent surface tinting based on normal
        vec3 albedo = 0.5 + 0.5 * cos(pos * 1.5 + vec3(0.0, 2.0, 4.0));
        vec3 color = albedo * (diff * 0.8 + 0.2) + vec3(spec * 0.7);

        // Depth fog
        color = mix(color, vec3(0.05, 0.05, 0.12), clamp(t_dist / 8.0, 0.0, 1.0));
        return vec4(color, 1.0);
    } else {
        // Space background gradient
        vec3 bg = mix(vec3(0.02, 0.02, 0.05), vec3(0.08, 0.05, 0.15), uv.y);
        return vec4(bg, 1.0);
    }
}

// ── Mode 4: Water Caustics / Ripple ───────────────────────────────────────
vec4 renderCaustics(vec2 uv, float t) {
    vec2 p = uv * 8.0;
    vec2 i = p;
    float c = 1.0;
    float inten = 0.005;

    for (int n = 0; n < 4; n++) {
        float tt = t * (1.0 - (3.5 / float(n + 1)));
        i = p + vec2(cos(tt - i.x) + sin(tt + i.y), sin(tt - i.y) + cos(tt + i.x));
        c += 1.0 / length(vec2(p.x / (sin(i.x + tt) / inten), p.y / (cos(i.y + tt) / inten)));
    }
    c /= 4.0;
    c = 1.17 - pow(c, 1.4);
    vec3 color = vec3(pow(abs(c), 8.0));
    color = clamp(color + vec3(0.0, 0.35, 0.65), 0.0, 1.0);

    return vec4(color, 1.0);
}

void main() {
    if (u_mode == 1) {
        FragColor = renderPlasma(vUV, u_time);
    } else if (u_mode == 2) {
        FragColor = renderCRT(vUV, u_time);
    } else if (u_mode == 3) {
        FragColor = renderRaymarchedGyroid(vUV, u_time);
    } else {
        FragColor = renderCaustics(vUV, u_time);
    }
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("         Project32 Engine - Duct Shader Playground          ");
    println!("    Custom Procedural Shaders, Uniforms & Hot-Reloading     ");
    println!("============================================================");

    let config = WindowConfig::new("Project32 — Duct Advanced Shaders")
        .size(1280, 720)
        .resizable(true);

    let mut window = Window::new(config)?;
    let (mut width, mut height) = window.inner_size();

    let mut current_backend = if std::env::args().any(|a| a == "--vk") {
        BackendPreference::Vulkan
    } else if std::env::args().any(|a| a == "--dx11") {
        BackendPreference::DirectX11
    } else if std::env::args().any(|a| a == "--dx12") {
        BackendPreference::DirectX12
    } else {
        BackendPreference::OpenGL
    };

    let test_mode = std::env::args().any(|a| a == "--test");

    println!("[Shaders] Initializing Duct graphics backend...");
    let mut duct = Duct::builder()
        .with_windowed(&window)
        .from_args()
        .vsync(true)
        .build()?;

    println!("[Shaders] Active Backend: {}", duct.backend_name());

    // ── Build Full-Screen Quad Geometry ───────────────────────────────────
    // Vertex format: Position (X, Y, Z), TexCoord (U, V) -> 5 floats (20 bytes stride)
    let quad_vertices: [f32; 20] = [
        // Pos           UV
        -1.0, -1.0, 0.0, 0.0, 0.0, // 0 Bottom-Left
         1.0, -1.0, 0.0, 1.0, 0.0, // 1 Bottom-Right
         1.0,  1.0, 0.0, 1.0, 1.0, // 2 Top-Right
        -1.0,  1.0, 0.0, 0.0, 1.0, // 3 Top-Left
    ];

    let quad_indices: [u16; 6] = [
        0, 1, 2,
        0, 2, 3,
    ];

    let vbo_bytes = unsafe {
        std::slice::from_raw_parts(
            quad_vertices.as_ptr() as *const u8,
            std::mem::size_of_val(&quad_vertices),
        )
    };
    let vbo = duct.create_buffer(DuctBufferType::Vertex, vbo_bytes.len(), Some(vbo_bytes), false)?;

    let ibo_bytes = unsafe {
        std::slice::from_raw_parts(
            quad_indices.as_ptr() as *const u8,
            std::mem::size_of_val(&quad_indices),
        )
    };
    let ibo = duct.create_buffer(DuctBufferType::Index, ibo_bytes.len(), Some(ibo_bytes), false)?;

    // ── Setup Custom Pipeline State ───────────────────────────────────────
    let vs = duct.create_shader(DuctShaderStage::Vertex, VS_SHADER_GLSL, Some("main"))?;
    let fs = duct.create_shader(DuctShaderStage::Fragment, FS_SHADER_GLSL, Some("main"))?;

    let layout = DuctVertexLayout::new(20)
        .add_attribute(0, DuctVertexFormat::Float3, 0)
        .add_attribute(1, DuctVertexFormat::Float2, 12);

    let pso = DuctPipelineStateDesc {
        vertex_shader: vs,
        fragment_shader: fs,
        vertex_layout: layout,
        cull_mode: DuctCullMode::None,
        blend_mode: DuctBlendMode::AlphaBlend,
        depth_test: false,
        depth_write: false,
        depth_compare: DuctCompareOp::Always,
    };
    duct.set_pipeline_state(&pso)?;

    println!("[Shaders] Controls:");
    println!("          [1] Cyberpunk Neon Plasma");
    println!("          [2] Retro CRT Monitor & Scanlines");
    println!("          [3] Raymarched 3D Gyroid / SDF Minimal Surface");
    println!("          [4] Water Caustics / Ripple Effect");
    println!("          [R] Hot-reload Fragment Shader");
    println!("          [B] Switch Graphics Backend");
    println!("          [Space] Pause / Resume Time");
    println!("          [ESC] Exit");

    let mut current_mode = 1i32;
    let mut animation_time = 0.0f32;
    let mut is_paused = false;
    let mut frame_count = 0u64;

    let _ = window.run(move |event, _win| -> ControlFlow {
        match event {
            Event::CloseRequested | Event::KeyDown { key: Key::Escape, .. } => {
                println!("[Shaders] Clean shutdown requested.");
                let _ = duct.destroy_buffer(vbo);
                let _ = duct.destroy_buffer(ibo);
                let _ = duct.destroy_shader(vs);
                let _ = duct.destroy_shader(fs);
                return ControlFlow::Exit;
            }
            Event::Resized { width: w, height: h } => {
                width = w.max(1);
                height = h.max(1);
                let _ = duct.resize_viewport(width, height);
            }
            Event::KeyDown { key, .. } => match key {
                Key::Num1 => {
                    current_mode = 1;
                    println!("[Shaders] Switched Mode to: 1 (Cyberpunk Neon Plasma)");
                }
                Key::Num2 => {
                    current_mode = 2;
                    println!("[Shaders] Switched Mode to: 2 (Retro CRT Monitor & Scanlines)");
                }
                Key::Num3 => {
                    current_mode = 3;
                    println!("[Shaders] Switched Mode to: 3 (Raymarched 3D Gyroid)");
                }
                Key::Num4 => {
                    current_mode = 4;
                    println!("[Shaders] Switched Mode to: 4 (Water Caustics)");
                }
                Key::Space => {
                    is_paused = !is_paused;
                    println!("[Shaders] Time animation paused: {}", is_paused);
                }
                Key::R => {
                    println!("[Shaders] Hot-reloading fragment shader...");
                    match duct.reload_shader(fs, DuctShaderStage::Fragment, FS_SHADER_GLSL, Some("main")) {
                        Ok(_) => println!("[Shaders] Fragment shader successfully reloaded!"),
                        Err(err) => eprintln!("[Shaders] Hot-reload error: {:?}", err),
                    }
                }
                Key::B => {
                    current_backend = match current_backend {
                        BackendPreference::OpenGL => BackendPreference::Vulkan,
                        BackendPreference::Vulkan => BackendPreference::DirectX11,
                        BackendPreference::DirectX11 => BackendPreference::DirectX12,
                        BackendPreference::DirectX12 => BackendPreference::OpenGL,
                        _ => BackendPreference::OpenGL,
                    };
                    println!("[Shaders] Switching backend to: {:?}", current_backend);
                    if let Err(e) = duct.switch_backend(current_backend) {
                        eprintln!("[Shaders] Failed switching backend: {:?}", e);
                    } else {
                        println!("[Shaders] Switched to backend: {}", duct.backend_name());
                        let _ = duct.resize_viewport(width, height);
                    }
                }
                _ => {}
            },
            Event::RedrawRequested => {
                if !is_paused {
                    animation_time += 0.016;
                }

                // Begin frame
                let _ = duct.begin_frame(DuctColor::BLACK);

                // Re-apply pipeline state
                let _ = duct.set_pipeline_state(&pso);

                // Update uniforms
                let _ = duct.set_uniform_float("u_time", animation_time);
                let _ = duct.set_uniform_vec2("u_resolution", width as f32, height as f32);
                let _ = duct.set_uniform_vec2("u_mouse", 0.5, 0.5);
                let _ = duct.set_uniform_int("u_mode", current_mode);

                // Bind geometry and draw full-screen quad
                let _ = duct.bind_vertex_buffer(vbo);
                let _ = duct.bind_index_buffer(ibo, DuctIndexFormat::U16);
                let _ = duct.draw_indexed(6, 0);

                let _ = duct.end_frame();
                let _ = duct.present();

                frame_count += 1;
                if test_mode && frame_count >= 10 {
                    println!("[Shaders] Automated test complete (rendered 10 frames).");
                    return ControlFlow::Exit;
                }
            }
            _ => {}
        }
        ControlFlow::Poll
    });

    println!("[Shaders] Shutting down Duct pipeline cleanly.");
    Ok(())
}
