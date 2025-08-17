use crate::{
    math::{affine_map::AffineMap, matrix2::Matrix2, matrix3::Matrix3, point::Point},
    painting::gl_garbage::{GlResource, gl_release},
};
use glow::{self, Context, HasContext, UniformLocation};
use log::warn;
use std::collections::HashMap;

pub struct VertexAttribDesc {
    components: i32,
    utype: u32, // VertexAttribPointerType
    normalized: bool,
}

#[allow(dead_code)]
impl VertexAttribDesc {
    pub const VEC2: VertexAttribDesc = VertexAttribDesc {
        components: 2,
        utype: glow::FLOAT,
        normalized: false,
    };

    pub const VEC3: VertexAttribDesc = VertexAttribDesc {
        components: 3,
        utype: glow::FLOAT,
        normalized: false,
    };

    pub const RGBA8: VertexAttribDesc = VertexAttribDesc {
        components: 4,
        utype: glow::UNSIGNED_BYTE,
        normalized: true,
    };

    pub const FLOAT: VertexAttribDesc = VertexAttribDesc {
        components: 1,
        utype: glow::FLOAT,
        normalized: false,
    };

    pub const U16: VertexAttribDesc = VertexAttribDesc {
        components: 1,
        utype: glow::UNSIGNED_SHORT,
        normalized: false,
    };

    pub const U32: VertexAttribDesc = VertexAttribDesc {
        components: 1,
        utype: glow::UNSIGNED_INT,
        normalized: false,
    };

    pub const I32: VertexAttribDesc = VertexAttribDesc {
        components: 1,
        utype: glow::INT,
        normalized: false,
    };
}

pub struct Shader {
    pub vertex_shader_source: String,
    pub fragment_shader_source: String,

    pub program: glow::Program,
    pub vertex_shader: glow::Shader,
    pub fragment_shader: glow::Shader,

    pub uniforms: HashMap<String, glow::ActiveUniform>,
    pub attributes: HashMap<String, glow::ActiveAttribute>,
}

impl Shader {
    pub fn compile_shader(gl: &glow::Context, source: &str, shader_type: u32) -> glow::Shader {
        unsafe {
            let shader = gl
                .create_shader(shader_type)
                .expect("Failed to create shader");
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
            let success = gl.get_shader_compile_status(shader);
            if !success {
                let info_log = gl.get_shader_info_log(shader);
                let shader_type_str = match shader_type {
                    glow::VERTEX_SHADER => "vertex shader",
                    glow::FRAGMENT_SHADER => "fragment shader",
                    glow::GEOMETRY_SHADER => "geometry shader",
                    _ => "unknown shader",
                };
                panic!("Error compiling {shader_type_str}: {}", info_log);
            }
            shader
        }
    }

    pub unsafe fn from_source(
        gl: &glow::Context,
        vertex_shader_source: &str,
        fragment_shader_source: &str,
    ) -> Shader {
        let vertex_shader = Shader::compile_shader(&gl, vertex_shader_source, glow::VERTEX_SHADER);
        let fragment_shader =
            Shader::compile_shader(&gl, fragment_shader_source, glow::FRAGMENT_SHADER);

        let program = gl.create_program().expect("Failed to create program");
        gl.attach_shader(program, vertex_shader);
        gl.attach_shader(program, fragment_shader);
        gl.link_program(program);

        let success = gl.get_program_link_status(program);
        if !success {
            let info_log = gl.get_program_info_log(program);
            panic!("Error linking program: {:?}", info_log);
        }

        let n_uniforms = gl.get_active_uniforms(program);
        let mut uniforms: HashMap<String, glow::ActiveUniform> = HashMap::new();
        for i in 0..n_uniforms {
            let active_uniform = gl
                .get_active_uniform(program, i)
                .expect("Failed to get active uniform");

            uniforms.insert(active_uniform.name.clone(), active_uniform);
        }

        let n_attributes = gl.get_active_attributes(program);
        let mut attributes: HashMap<String, glow::ActiveAttribute> = HashMap::new();
        for i in 0..n_attributes {
            let active_attribute = gl
                .get_active_attribute(program, i)
                .expect("Failed to get active attribute");

            attributes.insert(active_attribute.name.clone(), active_attribute);
        }

        Shader {
            vertex_shader_source: vertex_shader_source.to_string(),
            fragment_shader_source: fragment_shader_source.to_string(),
            program,
            vertex_shader,
            fragment_shader,
            uniforms,
            attributes,
        }
    }

    pub unsafe fn attribute_location(&self, gl: &glow::Context, name: &str) -> Option<u32> {
        let active_attribute = self.attributes.get(name)?;
        gl.get_attrib_location(self.program, &active_attribute.name)
    }

    pub unsafe fn assign_attribute_f32(
        &self,
        gl: &glow::Context,
        name: &str,
        desc: &VertexAttribDesc,
        offset: i32,
        stride: i32,
    ) {
        let Some(location) = self.attribute_location(gl, name) else {
            warn!("Attribute location for {name} not found.");
            return;
        };

        gl.enable_vertex_attrib_array(location);
        gl.vertex_attrib_pointer_f32(
            location,
            desc.components,
            desc.utype,
            desc.normalized,
            stride,
            offset,
        );
    }

    pub unsafe fn assign_attribute_i32(
        &self,
        gl: &glow::Context,
        name: &str,
        desc: &VertexAttribDesc,
        offset: i32,
        stride: i32,
    ) {
        let Some(location) = self.attribute_location(gl, name) else {
            warn!("Attribute location for {name} not found.");
            return;
        };

        gl.enable_vertex_attrib_array(location);
        gl.vertex_attrib_pointer_i32(location, desc.components, desc.utype, stride, offset);
    }

    pub unsafe fn use_program(&self, gl: &glow::Context) {
        gl.use_program(Some(self.program));
    }

    pub unsafe fn uniform_location<T: AssignUniform>(
        &self,
        gl: &glow::Context,
        location: &glow::UniformLocation,
        arg: T,
    ) {
        T::assign_uniform(gl, location, arg);
    }

    pub unsafe fn uniform<T: AssignUniform>(&self, gl: &glow::Context, name: &str, arg: T) {
        match self.uniforms.get(name) {
            None => {
                // println!("Uniform {name} does not exist in shader");
                // for name in self.uniforms.keys() {
                //     println!("existing name: {name}");
                // }
            }
            Some(active_uniform) => {
                let location = gl
                    .get_uniform_location(self.program, &active_uniform.name)
                    .expect("Failed to get uniform location");

                self.uniform_location(gl, &location, arg)
            }
        }
    }

    // inline void uniform(int location, std::array<float, 4> const& value)
    // {
    //     glUniform4f(location, value[0], value[1], value[2], value[3]);
    // }
    //
    // inline void uniform(int location, std::span<Vec2f const> value)
    // {
    //     glUniform2fv(location, (GLsizei)value.size(), &value[0].x);
    // }
    //
    // template<std::size_t N>
    // inline void uniform(int location, std::array<Vec2f, N> const& value)
    // {
    //     glUniform2fv(location, (GLsizei)value.size(), &value[0].x);
    // }
    //

    // inline void uniform(int location, Matrix2f const& value)
    // {
    //     // TODO: Check if works
    //     auto buf = value.entries();
    //     glUniformMatrix2fv(location, 1, true, buf.data());
    // }
    //
    // inline void uniform(int location, Matrix3f const& value)
    // {
    //     // TODO: Check if works
    //     auto buf = value.entries();
    //     glUniformMatrix3fv(location, 1, true, buf.data());
    // }
}

// pub trait From<T> {
//     fn from(T) -> Self;
// }

impl Drop for Shader {
    fn drop(&mut self) {
        gl_release(GlResource::Program(self.program));
        gl_release(GlResource::Shader(self.vertex_shader));
        gl_release(GlResource::Shader(self.fragment_shader));
    }
}

/// There's no impl for Rgba8 because it would be unclear if there is any color space conversion.
pub trait AssignUniform {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self);
}

impl AssignUniform for Point<f64> {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_2_f32(Some(location), value.x as f32, value.y as f32);
    }
}

impl AssignUniform for [f32; 4] {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_4_f32_slice(Some(location), &value);
    }
}

impl AssignUniform for &Matrix2<f64> {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_matrix_2_f32_slice(Some(location), true, &value.to_f32_array());
    }
}

impl AssignUniform for &Matrix3<f64> {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_matrix_3_f32_slice(Some(location), true, &value.cwise_as().to_array());
    }
}

impl AssignUniform for &AffineMap<f64> {
    unsafe fn assign_uniform(gl: &Context, location: &UniformLocation, value: Self) {
        AssignUniform::assign_uniform(gl, location, &Matrix3::from(*value))
    }
}

impl AssignUniform for &[Point<f64>] {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        // array of floats [x, y, x, y, ...]
        let mut buf = Vec::with_capacity(value.len() * 2);
        for v in value {
            buf.push(v.x as f32);
            buf.push(v.y as f32);
        }
        gl.uniform_2_f32_slice(Some(location), &buf);
    }
}

impl AssignUniform for f32 {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_1_f32(Some(location), value);
    }
}

impl AssignUniform for f64 {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_1_f32(Some(location), value as f32);
    }
}

impl AssignUniform for u32 {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_1_u32(Some(location), value);
    }
}

impl AssignUniform for i32 {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_1_i32(Some(location), value);
    }
}

impl AssignUniform for bool {
    unsafe fn assign_uniform(gl: &glow::Context, location: &glow::UniformLocation, value: Self) {
        gl.uniform_1_i32(Some(location), if value { 1 } else { 0 });
    }
}
