use crate::{
    math::{
        affine_map::AffineMap,
        arrow::Arrow,
        matrix3::Matrix3,
        pixel::{Side, SideName},
        point::Point,
    },
    painting::{
        gl_buffer::{GlBuffer, GlBufferTarget, GlVertexArrayObject},
        rect_vertices::RECT_TRIANGLE_INDICES,
        shader::{Shader, VertexAttribDesc},
    },
};
use glow::HasContext;
use image::{GrayImage, Luma};
use std::{
    array::from_fn,
    mem::{offset_of, size_of},
    path::Path,
};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct WaveVertex {
    pub line_start: [f32; 2],
    pub line_end: [f32; 2],
    pub quad_corner: u32,
}

#[derive(Debug, Clone, Default)]
pub struct WaveGeometry {
    pub vertices: Vec<WaveVertex>,
    pub indices: Vec<u32>,
}

impl WaveGeometry {
    pub fn add_arrows(&mut self, arrows: impl IntoIterator<Item = Arrow<f32>>) {
        for arrow in arrows {
            let line_length = arrow.length();
            // Kind of an arbitrary constant since we don't know what reference frame the
            // lines are in.
            assert!(line_length > 1e-10);

            let make_vertex = |quad_corner: usize| WaveVertex {
                line_start: arrow.a.to_array(),
                line_end: arrow.b.to_array(),
                quad_corner: quad_corner as u32,
            };

            let indices = RECT_TRIANGLE_INDICES.map(|i| i + self.vertices.len() as u32);
            self.indices.extend(indices);

            let vertices: [_; 4] = from_fn(make_vertex);
            self.vertices.extend(vertices);
        }
    }

    pub fn from_border(border: &[Side]) -> Self {
        let arrows = border
            .iter()
            .filter_map(|&side| Self::arrow_from_side(side))
            .map(Arrow::as_f32);

        let mut geometry = Self::default();
        geometry.add_arrows(arrows);
        geometry
    }

    pub fn arrow_from_side(side: Side) -> Option<Arrow<i64>> {
        let Point { x, y } = side.left_pixel;
        match side.name {
            SideName::Left => Some(Arrow::new_from([x, y], [x, y + 1])),
            SideName::Bottom => Some(Arrow::new_from([x, y + 1], [x + 1, y + 1])),
            SideName::BottomRight => None,
            SideName::Right => Some(Arrow::new_from([x + 1, y + 1], [x + 1, y])),
            SideName::Top => Some(Arrow::new_from([x + 1, y], [x, y])),
            SideName::TopLeft => None,
        }
    }
}

pub struct WavePainter {
    shader: Shader,
    array_buffer: GlBuffer<WaveVertex>,
    element_buffer: GlBuffer<u32>,
    vertex_array: GlVertexArrayObject,
}

impl WavePainter {
    pub unsafe fn new(gl: &glow::Context) -> Self {
        let vs_source = include_str!("shaders/wave.vert");
        let fs_source = include_str!("shaders/wave.frag");
        let shader = Shader::from_source(gl, &vs_source, &fs_source);

        // Create vertex, index buffers and assign to shader
        let array_buffer = GlBuffer::new(gl, GlBufferTarget::ArrayBuffer);
        let element_buffer = GlBuffer::new(gl, GlBufferTarget::ElementArrayBuffer);
        let vertex_array = GlVertexArrayObject::new(gl);

        vertex_array.bind(gl);
        array_buffer.bind(gl);
        element_buffer.bind(gl);

        let size = size_of::<WaveVertex>();

        shader.assign_attribute_f32(
            gl,
            "in_line_start",
            &VertexAttribDesc::VEC2,
            offset_of!(WaveVertex, line_start) as i32,
            size as i32,
        );

        shader.assign_attribute_f32(
            gl,
            "in_line_end",
            &VertexAttribDesc::VEC2,
            offset_of!(WaveVertex, line_end) as i32,
            size as i32,
        );

        shader.assign_attribute_i32(
            gl,
            "in_quad_corner",
            &VertexAttribDesc::I32,
            offset_of!(WaveVertex, quad_corner) as i32,
            size as i32,
        );

        Self {
            shader,
            array_buffer,
            element_buffer,
            vertex_array,
        }
    }

    pub unsafe fn draw_wave(
        &mut self,
        gl: &glow::Context,
        geometry: &WaveGeometry,
        view_from: AffineMap<f64>,
        device_from_view: AffineMap<f64>,
        wave_radius: f64,
    ) {
        // gl.enable(glow::BLEND);
        // gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        // gl.blend_equation(glow::FUNC_ADD);

        // The depth written by the fragment shader is the distance to the line, this way the
        // connection between line segments is properly drawn.
        gl.enable(glow::DEPTH_TEST);

        self.vertex_array.bind(gl);
        self.array_buffer.buffer_data(gl, &geometry.vertices);
        self.element_buffer.buffer_data(gl, &geometry.indices);

        self.shader.use_program(gl);

        let mat_device_from_view = Matrix3::from(device_from_view);
        self.shader
            .uniform(gl, "device_from_view", &mat_device_from_view);

        let mat_view_from = Matrix3::from(view_from);
        self.shader.uniform(gl, "view_from", &mat_view_from);

        self.shader.uniform(gl, "wave_radius", wave_radius);
        self.shader.uniform(gl, "line_width", wave_radius + 2.0);

        gl.draw_elements(
            glow::TRIANGLES,
            geometry.indices.len() as i32,
            glow::UNSIGNED_INT,
            0,
        );

        gl.disable(glow::DEPTH_TEST);

        // let (depth, w, h) = read_depth_full_viewport(gl);
        // save_depth_as_image(&depth, w as u32, h as u32, "wtf.png");
    }
}

// Reads depth from the currently bound READ_FRAMEBUFFER (or default FB if None bound).
// Returns row-major depth values (bottom-left origin, like OpenGL).
pub unsafe fn read_depth_region(
    gl: &glow::Context,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Vec<f32> {
    // Make sure you are reading from the framebuffer that has the depth you want.
    // If you render to an FBO, bind it to READ_FRAMEBUFFER before calling.
    // gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));

    // Pack alignment 1 to avoid row padding surprises.
    gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);

    // Allocate buffer (FLOAT = 4 bytes per pixel)
    let mut depth = vec![0.5f32; (width * height) as usize];

    let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut depth);

    while gl.get_error() != glow::NO_ERROR {
        // discard
    }

    // Read depth as floats
    println!("{x}, {y}, {width}, {height}");
    gl.read_pixels(
        x,
        y,
        width,
        height,
        glow::DEPTH_COMPONENT,
        glow::FLOAT,
        glow::PixelPackData::Slice(Some(bytes)),
    );

    check_gl_error(gl, "read_pixels");

    depth
}

// Convenience: read the whole viewport
pub unsafe fn read_depth_full_viewport(gl: &glow::Context) -> (Vec<f32>, usize, usize) {
    let mut vp = [0i32; 4];
    gl.get_parameter_i32_slice(glow::VIEWPORT, &mut vp); // [x, y, w, h]
    let floats = read_depth_region(gl, vp[0], vp[1], vp[2], vp[3]);
    (floats, vp[2] as usize, vp[3] as usize)
}

pub fn save_depth_as_image<P: AsRef<Path>>(
    depth: &[f32],
    width: u32,
    height: u32,
    path: P,
) -> image::ImageResult<()> {
    assert_eq!(depth.len(), (width * height) as usize);

    let mut img = GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let src_y = height - 1 - y; // flip vertically
            let idx = (src_y * width + x) as usize;
            let val = depth[idx].clamp(0.0, 1.0);
            let byte_val = (val * 255.0) as u8;
            img.put_pixel(x, y, Luma([byte_val]));
        }
    }

    img.save(path)
}

pub unsafe fn check_gl_error(gl: &glow::Context, label: &str) {
    loop {
        let err = gl.get_error();
        if err == glow::NO_ERROR {
            break;
        }

        let err_str = match err {
            glow::INVALID_ENUM => "INVALID_ENUM",
            glow::INVALID_VALUE => "INVALID_VALUE",
            glow::INVALID_OPERATION => "INVALID_OPERATION",
            glow::INVALID_FRAMEBUFFER_OPERATION => "INVALID_FRAMEBUFFER_OPERATION",
            glow::OUT_OF_MEMORY => "OUT_OF_MEMORY",
            glow::STACK_UNDERFLOW => "STACK_UNDERFLOW",
            glow::STACK_OVERFLOW => "STACK_OVERFLOW",
            _ => "UNKNOWN_ERROR",
        };

        eprintln!("[GL ERROR] {}: {}", label, err_str);
    }
}
