//! Zero-dependency SPIR-V 1.0 bytecode generator for `vkduct`.
//!
//! Dynamically constructs valid SPIR-V modules (`Vec<u32>`) for Vertex, Fragment,
//! and Compute shaders so that `vkduct` can compile and execute shaders on real
//! Vulkan hardware without requiring external tools (`glslc`, `shaderc`, etc.).
//!
//! Push Constant Layout (96 bytes total, shared across Vertex & Fragment stages):
//! - `offset 0`  (`vec4 uColor`):     RGBA color multiplier / uniform color (default `[1, 1, 1, 1]`)
//! - `offset 16` (`vec4 uParams`):    `xy` = 2D offset `(0, 0)`, `zw` = 2D scale `(1, 1)`
//! - `offset 32` (`mat4 uTransform`): 4x4 column-major transform matrix (default Identity)

#![allow(dead_code)]

const SPV_MAGIC: u32 = 0x0723_0203;
const SPV_VERSION_1_0: u32 = 0x0001_0000;

// Opcodes
const OP_CAPABILITY: u16 = 17;
const OP_MEMORY_MODEL: u16 = 14;
const OP_ENTRY_POINT: u16 = 15;
const OP_EXECUTION_MODE: u16 = 16;
const OP_DECORATE: u16 = 71;
const OP_MEMBER_DECORATE: u16 = 72;
const OP_TYPE_VOID: u16 = 19;
const OP_TYPE_INT: u16 = 21;
const OP_TYPE_FLOAT: u16 = 22;
const OP_TYPE_VECTOR: u16 = 23;
const OP_TYPE_MATRIX: u16 = 24;
const OP_TYPE_STRUCT: u16 = 30;
const OP_TYPE_POINTER: u16 = 32;
const OP_TYPE_FUNCTION: u16 = 33;
const OP_CONSTANT: u16 = 43;
const OP_CONSTANT_COMPOSITE: u16 = 44;
const OP_FUNCTION: u16 = 54;
const OP_FUNCTION_END: u16 = 56;
const OP_VARIABLE: u16 = 59;
const OP_LOAD: u16 = 61;
const OP_STORE: u16 = 62;
const OP_ACCESS_CHAIN: u16 = 65;
const OP_COMPOSITE_CONSTRUCT: u16 = 80;
const OP_COMPOSITE_EXTRACT: u16 = 81;
const OP_FNEGATE: u16 = 127;
const OP_FADD: u16 = 129;
const OP_FMUL: u16 = 133;
const OP_MATRIX_TIMES_VECTOR: u16 = 145;
const OP_LABEL: u16 = 248;
const OP_RETURN: u16 = 253;

// Enumerants
const CAPABILITY_SHADER: u32 = 1;
const ADDRESSING_MODEL_LOGICAL: u32 = 0;
const MEMORY_MODEL_GLSL450: u32 = 1;

const EXEC_MODEL_VERTEX: u32 = 0;
const EXEC_MODEL_FRAGMENT: u32 = 4;
const EXEC_MODEL_GL_COMPUTE: u32 = 5;

const EXEC_MODE_ORIGIN_UPPER_LEFT: u32 = 7;
const EXEC_MODE_LOCAL_SIZE: u32 = 17;

const STORAGE_CLASS_INPUT: u32 = 1;
const STORAGE_CLASS_OUTPUT: u32 = 3;
const STORAGE_CLASS_PUSH_CONSTANT: u32 = 9;

const DECORATION_BLOCK: u32 = 2;
const DECORATION_COL_MAJOR: u32 = 5;
const DECORATION_MATRIX_STRIDE: u32 = 7;
const DECORATION_BUILTIN: u32 = 11;
const DECORATION_LOCATION: u32 = 30;
const DECORATION_OFFSET: u32 = 35;

const BUILTIN_POSITION: u32 = 0;

// "main\0" encoded as two 32-bit little-endian words
const MAIN_STR_WORD_0: u32 = 0x6E69_616D; // 'm', 'a', 'i', 'n'
const MAIN_STR_WORD_1: u32 = 0x0000_0000; // '\0', 0, 0, 0

struct SpvBuilder {
    next_id: u32,
    capabilities: Vec<u32>,
    memory_model: Vec<u32>,
    entry_points: Vec<u32>,
    execution_modes: Vec<u32>,
    decorations: Vec<u32>,
    types_globals: Vec<u32>,
    functions: Vec<u32>,
}

impl SpvBuilder {
    fn new() -> Self {
        let mut b = Self {
            next_id: 1,
            capabilities: Vec::new(),
            memory_model: Vec::new(),
            entry_points: Vec::new(),
            execution_modes: Vec::new(),
            decorations: Vec::new(),
            types_globals: Vec::new(),
            functions: Vec::new(),
        };
        Self::push_op(&mut b.capabilities, OP_CAPABILITY, &[CAPABILITY_SHADER]);
        Self::push_op(
            &mut b.memory_model,
            OP_MEMORY_MODEL,
            &[ADDRESSING_MODEL_LOGICAL, MEMORY_MODEL_GLSL450],
        );
        b
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn push_op(target: &mut Vec<u32>, opcode: u16, operands: &[u32]) {
        let word_count = (1 + operands.len()) as u32;
        target.push((word_count << 16) | (opcode as u32));
        target.extend_from_slice(operands);
    }

    fn finish(self) -> Vec<u32> {
        let mut words = Vec::with_capacity(
            5 + self.capabilities.len()
                + self.memory_model.len()
                + self.entry_points.len()
                + self.execution_modes.len()
                + self.decorations.len()
                + self.types_globals.len()
                + self.functions.len(),
        );
        words.push(SPV_MAGIC);
        words.push(SPV_VERSION_1_0);
        words.push(0); // Generator
        words.push(self.next_id); // Bound
        words.push(0); // Schema
        words.extend_from_slice(&self.capabilities);
        words.extend_from_slice(&self.memory_model);
        words.extend_from_slice(&self.entry_points);
        words.extend_from_slice(&self.execution_modes);
        words.extend_from_slice(&self.decorations);
        words.extend_from_slice(&self.types_globals);
        words.extend_from_slice(&self.functions);
        words
    }
}

/// Builds a SPIR-V 1.0 Vertex Shader module:
/// - `location = 0`: `in vec3 inPos` (compatible with Float2, Float3, Float4 VBO layouts)
/// - `location = 1` (if `has_vertex_color`): `in vec4 inColor`
/// - PushConstants (96 bytes): `{ vec4 uColor; vec4 uParams; mat4 uTransform; }`
/// - Outputs `gl_Position` (with Y negated so +Y is UP, matching OpenGL) and `location = 0 out vec4 vColor`.
pub fn build_vertex_shader_spv(has_vertex_color: bool) -> Vec<u32> {
    let mut b = SpvBuilder::new();

    let main_id = b.alloc_id();
    let label_id = b.alloc_id();

    // Types
    let t_void = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_VOID, &[t_void]);

    let t_fn = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_FUNCTION, &[t_fn, t_void]);

    let t_float = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_FLOAT, &[t_float, 32]);

    let t_vec3 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_VECTOR, &[t_vec3, t_float, 3]);

    let t_vec4 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_VECTOR, &[t_vec4, t_float, 4]);

    let t_mat4 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_MATRIX, &[t_mat4, t_vec4, 4]);

    let t_int = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_INT, &[t_int, 32, 1]);

    // Constants
    let c_int_0 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_CONSTANT, &[t_int, c_int_0, 0]);

    let c_int_1 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_CONSTANT, &[t_int, c_int_1, 1]);

    let c_int_2 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_CONSTANT, &[t_int, c_int_2, 2]);

    let c_float_1 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_CONSTANT,
        &[t_float, c_float_1, 1.0f32.to_bits()],
    );

    let c_vec4_ones = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_CONSTANT_COMPOSITE,
        &[t_vec4, c_vec4_ones, c_float_1, c_float_1, c_float_1, c_float_1],
    );

    // gl_PerVertex output block
    let t_per_vertex = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_STRUCT, &[t_per_vertex, t_vec4]);
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_per_vertex, 0, DECORATION_BUILTIN, BUILTIN_POSITION],
    );
    SpvBuilder::push_op(&mut b.decorations, OP_DECORATE, &[t_per_vertex, DECORATION_BLOCK]);

    let t_ptr_out_per_vertex = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_out_per_vertex, STORAGE_CLASS_OUTPUT, t_per_vertex],
    );

    let var_per_vertex = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_VARIABLE,
        &[t_ptr_out_per_vertex, var_per_vertex, STORAGE_CLASS_OUTPUT],
    );

    // Input location 0: vec3 inPos
    let t_ptr_in_vec3 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_in_vec3, STORAGE_CLASS_INPUT, t_vec3],
    );

    let var_in_pos = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_VARIABLE,
        &[t_ptr_in_vec3, var_in_pos, STORAGE_CLASS_INPUT],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_DECORATE,
        &[var_in_pos, DECORATION_LOCATION, 0],
    );

    // Optional Input location 1: vec4 inColor
    let var_in_color = if has_vertex_color {
        let t_ptr_in_vec4 = b.alloc_id();
        SpvBuilder::push_op(
            &mut b.types_globals,
            OP_TYPE_POINTER,
            &[t_ptr_in_vec4, STORAGE_CLASS_INPUT, t_vec4],
        );
        let v = b.alloc_id();
        SpvBuilder::push_op(
            &mut b.types_globals,
            OP_VARIABLE,
            &[t_ptr_in_vec4, v, STORAGE_CLASS_INPUT],
        );
        SpvBuilder::push_op(&mut b.decorations, OP_DECORATE, &[v, DECORATION_LOCATION, 1]);
        Some(v)
    } else {
        None
    };

    // Output location 0: vec4 outColor (also used as pointer type for gl_Position access chain)
    let t_ptr_out_vec4 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_out_vec4, STORAGE_CLASS_OUTPUT, t_vec4],
    );

    let var_out_color = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_VARIABLE,
        &[t_ptr_out_vec4, var_out_color, STORAGE_CLASS_OUTPUT],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_DECORATE,
        &[var_out_color, DECORATION_LOCATION, 0],
    );

    // PushConstant struct: { vec4 uColor (0); vec4 uParams (16); mat4 uTransform (32); }
    let t_pc_struct = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_STRUCT,
        &[t_pc_struct, t_vec4, t_vec4, t_mat4],
    );
    SpvBuilder::push_op(&mut b.decorations, OP_DECORATE, &[t_pc_struct, DECORATION_BLOCK]);
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 0, DECORATION_OFFSET, 0],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 1, DECORATION_OFFSET, 16],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 2, DECORATION_OFFSET, 32],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 2, DECORATION_COL_MAJOR],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 2, DECORATION_MATRIX_STRIDE, 16],
    );

    let t_ptr_pc_struct = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_pc_struct, STORAGE_CLASS_PUSH_CONSTANT, t_pc_struct],
    );

    let t_ptr_pc_vec4 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_pc_vec4, STORAGE_CLASS_PUSH_CONSTANT, t_vec4],
    );

    let t_ptr_pc_mat4 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_pc_mat4, STORAGE_CLASS_PUSH_CONSTANT, t_mat4],
    );

    let var_pc = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_VARIABLE,
        &[t_ptr_pc_struct, var_pc, STORAGE_CLASS_PUSH_CONSTANT],
    );

    // Entry point declaration
    if let Some(in_col) = var_in_color {
        SpvBuilder::push_op(
            &mut b.entry_points,
            OP_ENTRY_POINT,
            &[
                EXEC_MODEL_VERTEX,
                main_id,
                MAIN_STR_WORD_0,
                MAIN_STR_WORD_1,
                var_per_vertex,
                var_in_pos,
                in_col,
                var_out_color,
            ],
        );
    } else {
        SpvBuilder::push_op(
            &mut b.entry_points,
            OP_ENTRY_POINT,
            &[
                EXEC_MODEL_VERTEX,
                main_id,
                MAIN_STR_WORD_0,
                MAIN_STR_WORD_1,
                var_per_vertex,
                var_in_pos,
                var_out_color,
            ],
        );
    }

    // Function main()
    SpvBuilder::push_op(&mut b.functions, OP_FUNCTION, &[t_void, main_id, 0, t_fn]);
    SpvBuilder::push_op(&mut b.functions, OP_LABEL, &[label_id]);

    let pos3 = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_LOAD, &[t_vec3, pos3, var_in_pos]);

    let px = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, px, pos3, 0]);
    let py = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, py, pos3, 1]);
    let pz = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, pz, pos3, 2]);

    let pos4 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.functions,
        OP_COMPOSITE_CONSTRUCT,
        &[t_vec4, pos4, px, py, pz, c_float_1],
    );

    let mat_ptr = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.functions,
        OP_ACCESS_CHAIN,
        &[t_ptr_pc_mat4, mat_ptr, var_pc, c_int_2],
    );
    let mat_val = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_LOAD, &[t_mat4, mat_val, mat_ptr]);

    let transformed = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.functions,
        OP_MATRIX_TIMES_VECTOR,
        &[t_vec4, transformed, mat_val, pos4],
    );

    let params_ptr = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.functions,
        OP_ACCESS_CHAIN,
        &[t_ptr_pc_vec4, params_ptr, var_pc, c_int_1],
    );
    let params_val = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_LOAD, &[t_vec4, params_val, params_ptr]);

    let off_x = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, off_x, params_val, 0]);
    let off_y = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, off_y, params_val, 1]);
    let scale_x = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, scale_x, params_val, 2]);
    let scale_y = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, scale_y, params_val, 3]);

    let tx = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, tx, transformed, 0]);
    let ty = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, ty, transformed, 1]);
    let tz = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, tz, transformed, 2]);
    let tw = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_COMPOSITE_EXTRACT, &[t_float, tw, transformed, 3]);

    let sx = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_FMUL, &[t_float, sx, tx, scale_x]);
    let sy = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_FMUL, &[t_float, sy, ty, scale_y]);

    let rx = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_FADD, &[t_float, rx, sx, off_x]);
    let ry = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_FADD, &[t_float, ry, sy, off_y]);

    // Flip Y so +Y is UP in Vulkan clip space (matching OpenGL convention)
    let neg_ry = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_FNEGATE, &[t_float, neg_ry, ry]);

    let final_pos = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.functions,
        OP_COMPOSITE_CONSTRUCT,
        &[t_vec4, final_pos, rx, neg_ry, tz, tw],
    );

    let gl_pos_ptr = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.functions,
        OP_ACCESS_CHAIN,
        &[t_ptr_out_vec4, gl_pos_ptr, var_per_vertex, c_int_0],
    );
    SpvBuilder::push_op(&mut b.functions, OP_STORE, &[gl_pos_ptr, final_pos]);

    if let Some(in_col) = var_in_color {
        let vc = b.alloc_id();
        SpvBuilder::push_op(&mut b.functions, OP_LOAD, &[t_vec4, vc, in_col]);
        SpvBuilder::push_op(&mut b.functions, OP_STORE, &[var_out_color, vc]);
    } else {
        SpvBuilder::push_op(&mut b.functions, OP_STORE, &[var_out_color, c_vec4_ones]);
    }

    SpvBuilder::push_op(&mut b.functions, OP_RETURN, &[]);
    SpvBuilder::push_op(&mut b.functions, OP_FUNCTION_END, &[]);

    b.finish()
}

/// Builds a SPIR-V 1.0 Fragment Shader module:
/// - `location = 0`: `in vec4 inColor`
/// - PushConstants (96 bytes): `{ vec4 uColor; vec4 uParams; mat4 uTransform; }`
/// - `location = 0`: `out vec4 FragColor` = `inColor * uColor`
pub fn build_fragment_shader_spv() -> Vec<u32> {
    let mut b = SpvBuilder::new();

    let main_id = b.alloc_id();
    let label_id = b.alloc_id();

    let t_void = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_VOID, &[t_void]);

    let t_fn = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_FUNCTION, &[t_fn, t_void]);

    let t_float = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_FLOAT, &[t_float, 32]);

    let t_vec4 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_VECTOR, &[t_vec4, t_float, 4]);

    let t_mat4 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_MATRIX, &[t_mat4, t_vec4, 4]);

    let t_int = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_INT, &[t_int, 32, 1]);

    let c_int_0 = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_CONSTANT, &[t_int, c_int_0, 0]);

    // Input location 0: vec4 inColor
    let t_ptr_in_vec4 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_in_vec4, STORAGE_CLASS_INPUT, t_vec4],
    );
    let var_in_color = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_VARIABLE,
        &[t_ptr_in_vec4, var_in_color, STORAGE_CLASS_INPUT],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_DECORATE,
        &[var_in_color, DECORATION_LOCATION, 0],
    );

    // Output location 0: vec4 outFragColor
    let t_ptr_out_vec4 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_out_vec4, STORAGE_CLASS_OUTPUT, t_vec4],
    );
    let var_out_color = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_VARIABLE,
        &[t_ptr_out_vec4, var_out_color, STORAGE_CLASS_OUTPUT],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_DECORATE,
        &[var_out_color, DECORATION_LOCATION, 0],
    );

    // PushConstant struct
    let t_pc_struct = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_STRUCT,
        &[t_pc_struct, t_vec4, t_vec4, t_mat4],
    );
    SpvBuilder::push_op(&mut b.decorations, OP_DECORATE, &[t_pc_struct, DECORATION_BLOCK]);
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 0, DECORATION_OFFSET, 0],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 1, DECORATION_OFFSET, 16],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 2, DECORATION_OFFSET, 32],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 2, DECORATION_COL_MAJOR],
    );
    SpvBuilder::push_op(
        &mut b.decorations,
        OP_MEMBER_DECORATE,
        &[t_pc_struct, 2, DECORATION_MATRIX_STRIDE, 16],
    );

    let t_ptr_pc_struct = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_pc_struct, STORAGE_CLASS_PUSH_CONSTANT, t_pc_struct],
    );

    let t_ptr_pc_vec4 = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_TYPE_POINTER,
        &[t_ptr_pc_vec4, STORAGE_CLASS_PUSH_CONSTANT, t_vec4],
    );

    let var_pc = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.types_globals,
        OP_VARIABLE,
        &[t_ptr_pc_struct, var_pc, STORAGE_CLASS_PUSH_CONSTANT],
    );

    // Entry point & execution mode
    SpvBuilder::push_op(
        &mut b.entry_points,
        OP_ENTRY_POINT,
        &[
            EXEC_MODEL_FRAGMENT,
            main_id,
            MAIN_STR_WORD_0,
            MAIN_STR_WORD_1,
            var_in_color,
            var_out_color,
        ],
    );
    SpvBuilder::push_op(
        &mut b.execution_modes,
        OP_EXECUTION_MODE,
        &[main_id, EXEC_MODE_ORIGIN_UPPER_LEFT],
    );

    // Function main()
    SpvBuilder::push_op(&mut b.functions, OP_FUNCTION, &[t_void, main_id, 0, t_fn]);
    SpvBuilder::push_op(&mut b.functions, OP_LABEL, &[label_id]);

    let in_col_val = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_LOAD, &[t_vec4, in_col_val, var_in_color]);

    let ucolor_ptr = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.functions,
        OP_ACCESS_CHAIN,
        &[t_ptr_pc_vec4, ucolor_ptr, var_pc, c_int_0],
    );
    let ucolor_val = b.alloc_id();
    SpvBuilder::push_op(&mut b.functions, OP_LOAD, &[t_vec4, ucolor_val, ucolor_ptr]);

    let final_col = b.alloc_id();
    SpvBuilder::push_op(
        &mut b.functions,
        OP_FMUL,
        &[t_vec4, final_col, in_col_val, ucolor_val],
    );

    SpvBuilder::push_op(&mut b.functions, OP_STORE, &[var_out_color, final_col]);
    SpvBuilder::push_op(&mut b.functions, OP_RETURN, &[]);
    SpvBuilder::push_op(&mut b.functions, OP_FUNCTION_END, &[]);

    b.finish()
}

/// Builds a SPIR-V 1.0 Compute Shader module (`local_size_x = 1, local_size_y = 1, local_size_z = 1`).
pub fn build_compute_shader_spv() -> Vec<u32> {
    let mut b = SpvBuilder::new();

    let main_id = b.alloc_id();
    let label_id = b.alloc_id();

    let t_void = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_VOID, &[t_void]);

    let t_fn = b.alloc_id();
    SpvBuilder::push_op(&mut b.types_globals, OP_TYPE_FUNCTION, &[t_fn, t_void]);

    SpvBuilder::push_op(
        &mut b.entry_points,
        OP_ENTRY_POINT,
        &[EXEC_MODEL_GL_COMPUTE, main_id, MAIN_STR_WORD_0, MAIN_STR_WORD_1],
    );
    SpvBuilder::push_op(
        &mut b.execution_modes,
        OP_EXECUTION_MODE,
        &[main_id, EXEC_MODE_LOCAL_SIZE, 1, 1, 1],
    );

    SpvBuilder::push_op(&mut b.functions, OP_FUNCTION, &[t_void, main_id, 0, t_fn]);
    SpvBuilder::push_op(&mut b.functions, OP_LABEL, &[label_id]);
    SpvBuilder::push_op(&mut b.functions, OP_RETURN, &[]);
    SpvBuilder::push_op(&mut b.functions, OP_FUNCTION_END, &[]);

    b.finish()
}
