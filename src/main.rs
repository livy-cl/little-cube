#![allow(unused_imports)]
#![allow(unused_variables)]

extern crate piston_window;
extern crate vecmath;
extern crate camera_controllers;
#[macro_use]
extern crate gfx;
extern crate shader_version;

use piston_window::*;
use gfx::traits::*;
use shader_version::Shaders;
use shader_version::glsl::GLSL;
use camera_controllers::{
    FirstPersonSettings,
    FirstPerson,
    CameraPerspective,
    model_view_projection
};


//----------------------------------------
// Cube associated data

gfx_vertex_struct!( Vertex {
    a_pos: [i8; 4] = "a_pos",
    // a_tex_coord contains the UV coordinates that the vertex is mapped to within the texture
    a_tex_coord: [i8; 2] = "a_tex_coord",
});

impl Vertex {
    fn new(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
        Vertex {
            a_pos: [pos[0], pos[1], pos[2], 1],
            a_tex_coord: tc,
        }
    }
}


gfx_pipeline!( pipe {
    vbuf: gfx::VertexBuffer<Vertex> = (),
    u_model_view_proj: gfx::Global<[[f32; 4]; 4]> = "u_model_view_proj",
    t_color: gfx::TextureSampler<[f32; 4]> = "t_color",
    out_color: gfx::RenderTarget<::gfx::format::Srgba8> = "o_Color",
    out_depth: gfx::DepthTarget<::gfx::format::DepthStencil> =
        gfx::preset::depth::LESS_EQUAL_WRITE,
});

//----------------------------------------

fn main() {
    let opengl = OpenGL::V3_2;

    let mut window: PistonWindow =
        WindowSettings::new("Little Cube", [640, 480])
            .exit_on_esc(true)
            .samples(4)
            .graphics_api(opengl)
            .build()
            .unwrap();
    window.set_capture_cursor(true);

    // GL resource factory
    let ref mut factory = window.factory.clone();

    // 8 corners of the cube
    let vertex_data = vec![
        // bottom
        Vertex::new([-1, -1, -1], [0, 0]),
        Vertex::new([-1, -1,  1], [0, 0]),
        Vertex::new([ 1, -1,  1], [0, 0]),
        Vertex::new([ 1, -1, -1], [0, 0]),

        // top
        Vertex::new([-1,  1, -1], [0, 0]),
        Vertex::new([-1,  1,  1], [0, 0]),
        Vertex::new([ 1,  1,  1], [0, 0]),
        Vertex::new([ 1,  1, -1], [0, 0]),

        // roof
        Vertex::new([ 0,  2,  1], [1, 1]),
        Vertex::new([ 0,  2, -1], [1, 1]),

        // floor
        Vertex::new([-20, -1,  20], [1, 0]),
        Vertex::new([ 20, -1,  20], [1, 0]),
        Vertex::new([ 20, -1, -20], [1, 0]),
        Vertex::new([-20, -1, -20], [1, 0]),
    ];

    // Creates triangles of the vertices. Great example:
    // https://gamedev.stackexchange.com/questions/68838/what-is-the-purpose-of-indices-in-3d-rendering
    let index_data: &[u16] = &[
        // cube
        5, 8, 9,    5, 4, 9, // roof left
        7, 6, 9,    6, 8, 9, // roof right
        4, 7, 9,    5, 8, 6, // roof sides

        2, 6, 7,    2, 7, 3, // right
        1, 5, 4,    4, 1, 0, // left
        0, 3, 7,    4, 7, 0, // front
        1, 2, 6,    1, 5, 6, // back

        10, 13, 12,     10, 11, 12, // floor
    ];

    // A Slice dictates in which and in what order vertices get processed.
    let (vbuf, slice) =
        factory.create_vertex_buffer_with_slice(&vertex_data, index_data);

    let texels = [
        [0xdb, 0x45, 0x00, 0x00],
        [0x45, 0x75, 0x00, 0x3b],
        [0x00, 0x00, 0x00, 0x00],
        [0x00, 0xff, 0x00, 0x00],
    ];
    let (_, texture_view) = factory.create_texture_immutable::<gfx::format::Rgba8>(
        gfx::texture::Kind::D2(2, 2, gfx::texture::AaMode::Single),
        gfx::texture::Mipmap::Provided,
        &[&texels]).unwrap();
    let (_, texture_view) = factory.create_texture_immutable::<gfx::format::Rgba8>(
        gfx::texture::Kind::D2(2, 2, gfx::texture::AaMode::Single),
        gfx::texture::Mipmap::Provided,
        &[&texels]).unwrap();

    let sinfo = gfx::texture::SamplerInfo::new(
        gfx::texture::FilterMethod::Bilinear,
        gfx::texture::WrapMode::Clamp);

    let glsl = opengl.to_glsl();
    let pso = factory.create_pipeline_simple(
        Shaders::new()
            .set(GLSL::V1_20, include_str!("../assets/shaders/cube_120.glslv"))
            .set(GLSL::V1_50, include_str!("../assets/shaders/cube_150.glslv"))
            .get(glsl).unwrap().as_bytes(),
        Shaders::new()
            .set(GLSL::V1_20, include_str!("../assets/shaders/cube_120.glslf"))
            .set(GLSL::V1_50, include_str!("../assets/shaders/cube_150.glslf"))
            .get(glsl).unwrap().as_bytes(),
        pipe::new()
    ).unwrap();

    let get_projection = |w: &PistonWindow| {
        let draw_size = w.window.draw_size();
        CameraPerspective {
            fov: 90.0, near_clip: 0.1, far_clip: 1000.0,
            aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32)
        }.projection()
    };

    let model = vecmath::mat4_id();
    let mut projection = get_projection(&window);
    let mut first_person = FirstPerson::new(
        [0.5, 0.5, 4.0],
        FirstPersonSettings::keyboard_wasd()
    );

    let mut data = pipe::Data {
        vbuf: vbuf.clone(),
        u_model_view_proj: [[0.0; 4]; 4],
        t_color: (texture_view, factory.create_sampler(sinfo)),
        out_color: window.output_color.clone(),
        out_depth: window.output_stencil.clone(),
    };

    while let Some(e) = window.next() {
        first_person.event(&e);

        window.draw_3d(&e, |window| {
            let args = e.render_args().unwrap();

            window.encoder.clear(&window.output_color, [0.3, 0.3, 0.3, 1.0]);
            window.encoder.clear_depth(&window.output_stencil, 1.0);

            data.u_model_view_proj = model_view_projection(
                model,
                first_person.camera(args.ext_dt).orthogonal(),
                projection
            );
            window.encoder.draw(&slice, &pso, &data);
        });

        if let Some(_) = e.resize_args() {
            projection = get_projection(&window);
            data.out_color = window.output_color.clone();
            data.out_depth = window.output_stencil.clone();
        }
    }
}