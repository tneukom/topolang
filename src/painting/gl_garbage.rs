use glow::HasContext;
use log::warn;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub enum GlResource {
    Texture(glow::Texture),
    Buffer(glow::Buffer),
    Program(glow::Program),
    Shader(glow::Shader),
    VertexArray(glow::VertexArray),
}

static GARBAGE: Mutex<Vec<GlResource>> = Mutex::new(Vec::new());

pub fn gl_release(resource: GlResource) {
    let mut garbage = GARBAGE.lock().unwrap();
    garbage.push(resource);
}

pub unsafe fn gl_gc(gl: &glow::Context) {
    let mut garbage = GARBAGE.lock().unwrap();
    for resource in garbage.drain(..) {
        warn!("delete texture {:?}", resource);
        match resource {
            GlResource::Texture(texture) => gl.delete_texture(texture),
            GlResource::Buffer(buffer) => gl.delete_buffer(buffer),
            GlResource::Program(program) => gl.delete_program(program),
            GlResource::Shader(shader) => gl.delete_shader(shader),
            GlResource::VertexArray(vertex_array) => gl.delete_vertex_array(vertex_array),
        }
    }
}
