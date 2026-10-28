//! Shader cross-compilation pipeline using Naga.
//!
//! Provides translation from GLSL source into:
//! - SPIR-V binary words (for Vulkan / vkduct)
//! - HLSL source code (for Direct3D 11 & 12)
//!
//! With formatted error reporting that includes line numbers and source snippets.

use crate::DuctShaderStage;

#[derive(Debug)]
pub struct ShaderCompileError {
    pub message: String,
}

impl std::fmt::Display for ShaderCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ShaderCompileError {}

/// Preprocesses legacy or desktop GLSL (e.g. #version 330 core, loose uniforms, unannotated outputs)
/// to be compatible with Vulkan GLSL 450 required by Naga.
pub fn preprocess_glsl(source: &str, stage: DuctShaderStage, for_spirv_push_constants: bool) -> String {
    let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    let mut has_version = false;
    let mut loose_uniforms = Vec::new();
    let mut other_uniform_globals = Vec::new();
    let mut processed_lines = Vec::new();

    let mut out_loc = 0;
    let mut sampler_binding = 0;

    for line in lines {
        let trimmed = line.trim();

        // 1. Normalize version to 450
        if trimmed.starts_with("#version") {
            has_version = true;
            processed_lines.push("#version 450".to_string());
            continue;
        }

        // 2. Assign locations to varyings (vertex outputs, fragment inputs/outputs)
        if stage == DuctShaderStage::Fragment {
            if trimmed.starts_with("out ") && !trimmed.contains("layout") {
                processed_lines.push(format!("layout(location = {}) {}", out_loc, trimmed));
                out_loc += 1;
                continue;
            } else if trimmed.starts_with("in ") && !trimmed.contains("layout") {
                let name = trimmed.split_whitespace().last().unwrap_or("").trim_end_matches(';');
                let loc = match name {
                    "vColor" => 0,
                    "vUv" | "vUV" => 1,
                    "vFragPos" => 2,
                    "vNormal" => 3,
                    "vDepth" => 4,
                    _ => out_loc,
                };
                processed_lines.push(format!("layout(location = {}) {}", loc, trimmed));
                continue;
            }
        } else if stage == DuctShaderStage::Vertex {
            if trimmed.starts_with("out ") && !trimmed.contains("layout") {
                let name = trimmed.split_whitespace().last().unwrap_or("").trim_end_matches(';');
                let loc = match name {
                    "vColor" => 0,
                    "vUv" | "vUV" => 1,
                    "vFragPos" => 2,
                    "vNormal" => 3,
                    "vDepth" => 4,
                    _ => out_loc,
                };
                processed_lines.push(format!("layout(location = {}) {}", loc, trimmed));
                out_loc += 1;
                continue;
            }
        }

        // 3. Collect loose uniforms (e.g. uniform vec4 uColor;)
        if trimmed.starts_with("uniform ") && trimmed.ends_with(';') && !trimmed.contains('{') && !trimmed.contains('(') {
            let decl = trimmed.trim_start_matches("uniform").trim();
            // Don't pack samplers into uniform blocks. Naga GLSL requires separated texture & sampler.
            if decl.starts_with("sampler") {
                let clean = decl.trim_end_matches(';').trim();
                let parts: Vec<&str> = clean.split_whitespace().collect();
                if parts.len() >= 2 {
                    let samp_ty = parts[0];
                    let var_name = parts[1];
                    let tex_ty = match samp_ty {
                        "sampler2D" => "texture2D",
                        "sampler3D" => "texture3D",
                        "samplerCube" => "textureCube",
                        "sampler1D" => "texture1D",
                        _ => "texture2D",
                    };
                    let b = sampler_binding;
                    sampler_binding += 2;
                    processed_lines.push(format!("layout(set = 0, binding = {}) uniform {} {}_tex;", b, tex_ty, var_name));
                    processed_lines.push(format!("layout(set = 0, binding = {}) uniform sampler {}_smp;", b + 1, var_name));
                    processed_lines.push(format!("#define {} {}({}_tex, {}_smp)", var_name, samp_ty, var_name, var_name));
                } else {
                    processed_lines.push(format!("layout(binding = 0) uniform {}", decl));
                }
            } else if for_spirv_push_constants {
                // If it's a known push constant uniform name, it's covered by _DuctPushConstants
                let is_known_pc = decl.contains("uColor")
                    || decl.contains("uParams")
                    || decl.contains("uTransform")
                    || decl.contains("uExtra")
                    || decl.contains("uMVP")
                    || decl.contains("uModel")
                    || decl.contains("uObjectColor");

                if !is_known_pc {
                    other_uniform_globals.push(decl.to_string());
                }
            } else {
                loose_uniforms.push(format!("    {}", decl));
            }
            continue;
        }

        processed_lines.push(line);
    }

    fn default_init_for_decl(decl: &str) -> String {
        let clean = decl.trim().trim_end_matches(';').trim();
        let parts: Vec<&str> = clean.split_whitespace().collect();
        let filtered: Vec<&str> = parts
            .into_iter()
            .filter(|&p| p != "highp" && p != "mediump" && p != "lowp")
            .collect();
        if filtered.len() >= 2 {
            let ty = filtered[0];
            let name = filtered[1];
            let init = match ty {
                "float" => "0.0",
                "int" => "0",
                "uint" => "0u",
                "bool" => "false",
                "vec2" => "vec2(0.0)",
                "vec3" => "vec3(1.0)",
                "vec4" => "vec4(1.0)",
                "mat3" => "mat3(1.0)",
                "mat4" => "mat4(1.0)",
                _ => "0.0",
            };
            format!("{} {} = {};", ty, name, init)
        } else {
            format!("{};", clean)
        }
    }

    let mut result = String::new();
    let mut inserted_uniforms = false;

    let emit_uniforms = |res: &mut String| {
        if for_spirv_push_constants {
            res.push_str(
                r#"
layout(push_constant) uniform _DuctPushConstants {
    vec4 uColor;
    vec4 uParams;
    mat4 uTransform;
    vec4 uExtra;
};
#define uMVP uTransform
#define uModel uTransform
#define uObjectColor uColor
"#,
            );
            for g in &other_uniform_globals {
                res.push_str(&default_init_for_decl(g));
                res.push('\n');
            }
        } else if !loose_uniforms.is_empty() {
            res.push_str("layout(binding = 0) uniform _DuctLooseUniforms {\n");
            for u in &loose_uniforms {
                res.push_str(u);
                res.push('\n');
            }
            res.push_str("};\n");
        }
    };

    if !has_version {
        result.push_str("#version 450\n");
        emit_uniforms(&mut result);
        inserted_uniforms = true;
    }

    for line in processed_lines {
        result.push_str(&line);
        result.push('\n');

        // Insert uniforms right after #version
        if !inserted_uniforms && line.starts_with("#version") {
            emit_uniforms(&mut result);
            inserted_uniforms = true;
        }
    }

    result
}

pub fn map_stage_to_naga(stage: DuctShaderStage) -> naga::ShaderStage {
    match stage {
        DuctShaderStage::Vertex => naga::ShaderStage::Vertex,
        DuctShaderStage::Fragment => naga::ShaderStage::Fragment,
        DuctShaderStage::Compute => naga::ShaderStage::Compute,
    }
}

/// Parses GLSL shader source into a validated Naga module.
pub fn compile_glsl_to_naga(
    source: &str,
    stage: DuctShaderStage,
    for_spirv_push_constants: bool,
) -> Result<(naga::Module, naga::valid::ModuleInfo), ShaderCompileError> {
    let clean_source = preprocess_glsl(source, stage, for_spirv_push_constants);
    let naga_stage = map_stage_to_naga(stage);
    let options = naga::front::glsl::Options::from(naga_stage);
    let mut frontend = naga::front::glsl::Frontend::default();

    let module = frontend.parse(&options, &clean_source).map_err(|errors| {
        let mut msg = format!("GLSL {:?} compilation failed:\n", stage);
        msg.push_str(&errors.emit_to_string(&clean_source));
        ShaderCompileError { message: msg }
    })?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );

    let info = validator.validate(&module).map_err(|err| ShaderCompileError {
        message: format!("GLSL {:?} validation failed: {}\n", stage, err),
    })?;

    Ok((module, info))
}

/// Compiles GLSL shader source to SPIR-V u32 words.
pub fn glsl_to_spirv(
    source: &str,
    stage: DuctShaderStage,
) -> Result<Vec<u32>, ShaderCompileError> {
    let (module, info) = compile_glsl_to_naga(source, stage, true)?;
    let spv_options = naga::back::spv::Options::default();
    naga::back::spv::write_vec(&module, &info, &spv_options, None).map_err(|err| {
        ShaderCompileError {
            message: format!("SPIR-V emission failed: {}\n", err),
        }
    })
}

/// Compiles GLSL shader source to HLSL string for Direct3D 11 / Direct3D 12.
pub fn glsl_to_hlsl(
    source: &str,
    stage: DuctShaderStage,
) -> Result<String, ShaderCompileError> {
    let (module, info) = compile_glsl_to_naga(source, stage, false)?;
    let mut hlsl_options = naga::back::hlsl::Options::default();
    hlsl_options.shader_model = naga::back::hlsl::ShaderModel::V5_0;
    hlsl_options.fake_missing_bindings = true;

    let mut hlsl_out = String::new();
    let mut writer = naga::back::hlsl::Writer::new(&mut hlsl_out, &hlsl_options);
    writer.write(&module, &info, None).map_err(|err| ShaderCompileError {
        message: format!("HLSL emission failed: {}\n", err),
    })?;

    Ok(hlsl_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_VS: &str = r#"#version 450
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_texcoord;

layout(location = 0) out vec2 v_texcoord;

void main() {
    v_texcoord = a_texcoord;
    gl_Position = vec4(a_position, 1.0);
}
"#;

    const TEST_FS: &str = r#"#version 450
layout(location = 0) in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

void main() {
    fragColor = vec4(v_texcoord, 0.0, 1.0);
}
"#;

    #[test]
    fn test_spirv_and_hlsl_basic() {
        let spv_vs = glsl_to_spirv(TEST_VS, DuctShaderStage::Vertex).expect("Basic VS to SPIR-V");
        assert_eq!(spv_vs[0], 0x07230203);
        let spv_fs = glsl_to_spirv(TEST_FS, DuctShaderStage::Fragment).expect("Basic FS to SPIR-V");
        assert_eq!(spv_fs[0], 0x07230203);

        let hlsl_vs = glsl_to_hlsl(TEST_VS, DuctShaderStage::Vertex).expect("Basic VS to HLSL");
        assert!(hlsl_vs.contains("main"));
        let hlsl_fs = glsl_to_hlsl(TEST_FS, DuctShaderStage::Fragment).expect("Basic FS to HLSL");
        assert!(hlsl_fs.contains("main"));
    }

    #[test]
    fn test_cube3d_shaders() {
        let vs = r#"
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
        let fs = r#"
            #version 330 core
            in vec3 vColor;
            in vec2 vUv;
            out vec4 FragColor;

            void main() {
                FragColor = vec4(vColor, 1.0);
            }
        "#;
        let spv_vs = glsl_to_spirv(vs, DuctShaderStage::Vertex).expect("Cube3D VS to SPIR-V");
        assert_eq!(spv_vs[0], 0x07230203);
        let spv_fs = glsl_to_spirv(fs, DuctShaderStage::Fragment).expect("Cube3D FS to SPIR-V");
        assert_eq!(spv_fs[0], 0x07230203);
    }

    #[test]
    fn test_sandbox_shaders() {
        let vs = r#"
            #version 450
            layout (location = 0) in vec3 aPos;
            void main() {
                gl_Position = vec4(aPos, 1.0);
            }
        "#;
        let fs = r#"
            #version 450
            out vec4 FragColor;
            uniform vec4 uColor;
            void main() {
                FragColor = uColor;
            }
        "#;

        let spv_vs = glsl_to_spirv(vs, DuctShaderStage::Vertex).expect("Sandbox VS to SPIR-V");
        assert_eq!(spv_vs[0], 0x07230203);

        let hlsl_vs = glsl_to_hlsl(vs, DuctShaderStage::Vertex).expect("Sandbox VS to HLSL");
        assert!(hlsl_vs.contains("SV_Position"));

        let hlsl_fs = glsl_to_hlsl(fs, DuctShaderStage::Fragment).expect("Sandbox FS to HLSL");
        assert!(hlsl_fs.contains("SV_Target"));
        println!("FS HLSL:\n{}", hlsl_fs);
    }

    #[test]
    fn test_texture_glsl() {
        let fs = r#"
            #version 330 core
            in vec2 vUV;
            in vec4 vColor;
            out vec4 FragColor;

            uniform sampler2D uFontTexture;

            void main() {
                vec4 tex = texture(uFontTexture, vUV);
                FragColor = vec4(vColor.rgb, vColor.a * tex.a);
            }
        "#;
        let spv = glsl_to_spirv(fs, DuctShaderStage::Fragment).expect("Texture FS to SPIR-V");
        assert_eq!(spv[0], 0x07230203);

        let hlsl = glsl_to_hlsl(fs, DuctShaderStage::Fragment).expect("Texture FS to HLSL");
        assert!(hlsl.contains("Sample"));
    }
}
