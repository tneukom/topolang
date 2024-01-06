use std::{marker::PhantomData, sync::Arc};

use glow::HasContext;

#[derive(Copy, Clone, Debug)]
pub enum GlBufferTarget {
    ArrayBuffer,
    ElementArrayBuffer,
}

impl GlBufferTarget {
    pub fn to_gl_enum(self) -> u32 {
        match self {
            GlBufferTarget::ArrayBuffer => glow::ARRAY_BUFFER,
            GlBufferTarget::ElementArrayBuffer => glow::ELEMENT_ARRAY_BUFFER,
        }
    }
}

pub struct GlBuffer<T> {
    pub id: glow::Buffer,
    pub target: GlBufferTarget,

    context: Arc<glow::Context>,

    phantom: PhantomData<T>,
}

impl<T> GlBuffer<T> {
    pub unsafe fn new(context: Arc<glow::Context>, target: GlBufferTarget) -> GlBuffer<T> {
        let id = context.create_buffer().expect("Cannot create buffer");
        // context.bind_buffer(target, Some(&id));
        // context.buffer_data_u8(target, &[], glow::STATIC_DRAW);
        // context.bind_buffer(target, None);
        GlBuffer {
            id,
            target,
            context,
            phantom: PhantomData,
        }
    }

    pub unsafe fn buffer_data(&self, data: &[T]) {
        // data as slice of u8
        let data_bytes = data.align_to::<u8>().1;
        let gl_target = self.target.to_gl_enum();
        self.context.bind_buffer(gl_target, Some(self.id));
        self.context
            .buffer_data_u8_slice(gl_target, &data_bytes, glow::STATIC_DRAW);
    }

    pub unsafe fn bind(&self) {
        self.context
            .bind_buffer(self.target.to_gl_enum(), Some(self.id));
    }

    pub unsafe fn unbind(&self) {
        self.context.bind_buffer(self.target.to_gl_enum(), None);
    }
}

impl<T> Drop for GlBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            self.context.delete_buffer(self.id);
        }
    }
}

pub struct GlVertexArray {
    pub id: glow::VertexArray,
    context: Arc<glow::Context>,
}

impl GlVertexArray {
    pub unsafe fn new(context: Arc<glow::Context>) -> GlVertexArray {
        let id = context
            .create_vertex_array()
            .expect("Cannot create vertex array");
        GlVertexArray { id, context }
    }

    pub unsafe fn bind(&self) {
        self.context.bind_vertex_array(Some(self.id));
    }

    pub unsafe fn unbind(&self) {
        self.context.bind_vertex_array(None);
    }
}

impl Drop for GlVertexArray {
    fn drop(&mut self) {
        unsafe {
            self.context.delete_vertex_array(self.id);
        }
    }
}
