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
pub fn preprocess_glsl(source: &str, stage: DuctShaderStage) -> String {
    let lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
    let mut has_version = false;
    let mut loose_uniforms = Vec::new();
    let mut processed_lines = Vec::new();

    let mut out_loc = 0;

    for line in lines {
        let trimmed = line.trim();

        // 1. Normalize version to 450
        if trimmed.starts_with("#version") {
            has_version = true;
            processed_lines.push("#version 450".to_string());
            continue;
        }

        // 2. Fragment shader: if 'out vec4 ...' has no layout(location = ...), add layout(location = 0)
        if stage == DuctShaderStage::Fragment
            && trimmed.starts_with("out ")
            && !trimmed.contains("layout")
        {
            processed_lines.push(format!("layout(location = {}) {}", out_loc, trimmed));
            out_loc += 1;
            continue;
        }

        // 3. Collect loose uniforms (e.g. uniform vec4 uColor;) to pack into a default uniform buffer
        if trimmed.starts_with("uniform ") && trimmed.ends_with(';') && !trimmed.contains('{') && !trimmed.contains('(') {
            let decl = trimmed.trim_start_matches("uniform").trim();
            // Don't pack samplers (sampler2D, samplerCube, etc.) into uniform buffers!
            if decl.starts_with("sampler") {
                processed_lines.push(format!("layout(binding = 0) uniform {}", decl));
            } else {
                loose_uniforms.push(format!("    {}", decl));
            }
            continue;
        }

        processed_lines.push(line);
    }

    let mut result = String::new();
    if !has_version {
        result.push_str("#version 450\n");
    }

    let mut inserted_uniforms = false;
    for line in processed_lines {
        result.push_str(&line);
        result.push('\n');

        // Insert packed uniform block right after #version
        if !inserted_uniforms && (line.starts_with("#version") || !has_version) {
            if !loose_uniforms.is_empty() {
                result.push_str("layout(binding = 0) uniform _DuctLooseUniforms {\n");
                for u in &loose_uniforms {
                    result.push_str(u);
                    result.push('\n');
                }
                result.push_str("};\n");
            }
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
) -> Result<(naga::Module, naga::valid::ModuleInfo), ShaderCompileError> {
    let clean_source = preprocess_glsl(source, stage);
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
    let (module, info) = compile_glsl_to_naga(source, stage)?;
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
    let (module, info) = compile_glsl_to_naga(source, stage)?;
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
    fn test_glsl_to_spirv() {
        let spv_vs = glsl_to_spirv(TEST_VS, DuctShaderStage::Vertex).expect("VS to SPIR-V failed");
        assert!(!spv_vs.is_empty());
        assert_eq!(spv_vs[0], 0x07230203); // SPIR-V magic number

        let spv_fs = glsl_to_spirv(TEST_FS, DuctShaderStage::Fragment).expect("FS to SPIR-V failed");
        assert!(!spv_fs.is_empty());
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
}
