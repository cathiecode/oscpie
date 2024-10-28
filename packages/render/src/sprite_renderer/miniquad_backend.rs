use miniquad::{self, BlendFactor, BlendState, BlendValue};

use super::*;

#[derive(Debug, Clone)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

static STANDARD_VERTICES: [Vertex; 4] = [
    // Left Top
    Vertex {
        pos: Vec2(-0.5, -0.5),
        uv: Vec2(0., 1.),
    },
    // Right Top
    Vertex {
        pos: Vec2(0.5, -0.5),
        uv: Vec2(1., 1.),
    },
    // Right Bottom
    Vertex {
        pos: Vec2(0.5, 0.5),
        uv: Vec2(1., 0.),
    },
    // Left Bottom
    Vertex {
        pos: Vec2(-0.5, 0.5),
        uv: Vec2(0., 0.),
    },
];

static STANDARD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

// OPTIMIZE
fn sprite_vertices(mat: &Mat3x3, reference: &mut [Vertex]) {
    let mut i = 0;
    for vertex in reference.iter_mut() {
        vertex.pos = mat * STANDARD_VERTICES[i].pos;
        vertex.uv = STANDARD_VERTICES[i].uv;
        i += 1;
    }
}

fn sprites_indices(sprite_count: usize) -> Vec<u16> {
    let mut idndices: Vec<u16> = Vec::with_capacity(sprite_count * 6);

    for j in 0..sprite_count as u16 {
        for i in &STANDARD_INDICES {
            idndices.push(j * 4 + i);
        }
    }

    idndices
}

pub struct MiniquadBackendEventHandler<E>
where
    E: EventHandler<MiniquadRenderingBackend>,
{
    event_handler: E,
    pipeline: miniquad::Pipeline,
    backend: MiniquadRenderingBackend,
}

impl<E> MiniquadBackendEventHandler<E>
where
    E: EventHandler<MiniquadRenderingBackend>,
{
    pub fn new<F>(event_handler: F) -> Self
    where
        F: FnOnce(&mut MiniquadRenderingBackend) -> E,
    {
        let mut ctx: Box<dyn miniquad::RenderingBackend> =
            miniquad::window::new_rendering_backend();

        // let texture = ctx.new_texture_from_rgba8(4, 4, &pixels);

        let shader = ctx
            .new_shader(
                match ctx.info().backend {
                    miniquad::Backend::OpenGl => miniquad::ShaderSource::Glsl {
                        vertex: shader::VERTEX,
                        fragment: shader::FRAGMENT,
                    },
                    miniquad::Backend::Metal => miniquad::ShaderSource::Msl {
                        program: shader::METAL,
                    },
                },
                shader::meta(),
            )
            .unwrap();

        let pipeline = ctx.new_pipeline(
            &[miniquad::BufferLayout::default()],
            &[
                miniquad::VertexAttribute::new("in_pos", miniquad::VertexFormat::Float2),
                miniquad::VertexAttribute::new("in_uv", miniquad::VertexFormat::Float2),
            ],
            shader,
            miniquad::PipelineParams {
                color_blend: Some(BlendState::new(
                    miniquad::Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..miniquad::PipelineParams::default()
            },
        );

        let mut backend = MiniquadRenderingBackend { ctx };

        let event_handler = event_handler(&mut backend);

        MiniquadBackendEventHandler {
            event_handler,
            backend,
            pipeline,
        }
    }
}

impl<E> miniquad::EventHandler for MiniquadBackendEventHandler<E>
where
    E: EventHandler<MiniquadRenderingBackend>,
{
    fn update(&mut self) {
        self.event_handler.update();
    }

    fn draw(&mut self) {
        self.backend.ctx.begin_default_pass(Default::default());

        self.backend.ctx.apply_pipeline(&self.pipeline);

        let render_graph = self.event_handler.draw(&mut self.backend);

        for pass in render_graph.pass.iter() {
            // OPTIMIZE: Reuse binding

            let mut vertex_array: Vec<Vertex> = Vec::with_capacity(pass.sprites.len() * 4);

            unsafe {
                vertex_array.set_len(pass.sprites.len() * 4);

                for (i, sprite) in pass.sprites.iter().enumerate() {
                    sprite_vertices(&sprite.matrix, &mut vertex_array[i * 4..i * 4 + 4]);
                }
            }

            let vertex_buffer = self.backend.ctx.new_buffer(
                miniquad::BufferType::VertexBuffer,
                miniquad::BufferUsage::Immutable,
                miniquad::BufferSource::slice::<Vertex>(&vertex_array),
            );

            let indices = sprites_indices(pass.sprites.len());

            let indice_length = indices.len();

            let index_buffer = self.backend.ctx.new_buffer(
                miniquad::BufferType::IndexBuffer,
                miniquad::BufferUsage::Immutable,
                miniquad::BufferSource::slice(&indices),
            );

            let bindings = miniquad::Bindings {
                vertex_buffers: vec![vertex_buffer],
                index_buffer: index_buffer,
                images: vec![pass.texture],
            };

            self.backend.ctx.apply_bindings(&bindings);

            self.backend.ctx.draw(0, indice_length as i32, 1);
        }

        self.backend.ctx.end_render_pass();

        self.backend.ctx.commit_frame();
    }
}

pub struct MiniquadRenderingBackend {
    ctx: Box<dyn miniquad::RenderingBackend>,
}

impl RenderingBackend for MiniquadRenderingBackend {
    type Texture = miniquad::TextureId;
    type Sprite = ();

    fn texture_from_raw_rgba_u8(&mut self, width: u16, height: u16, data: &[u8]) -> Self::Texture {
        self.ctx
            .new_texture_from_rgba8(width as u16, height as u16, data)
    }

    fn get_stage_size(&self) -> Vec2 {
        Vec2(100.0, 100.0)
    }
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 150
    in vec2 in_pos;
    in vec2 in_uv;

    uniform vec2 offset;

    out lowp vec2 texcoord;

    void main() {
        gl_Position = vec4(in_pos + offset, 0, 1);
        texcoord = in_uv;
    }"#;

    pub const FRAGMENT: &str = r#"#version 150
    in lowp vec2 texcoord;

    uniform sampler2D tex;

    out vec4 frag_color;

    void main() {
        frag_color = texture2D(tex, texcoord);
    }"#;

    pub const METAL: &str = r#"
    #include <metal_stdlib>

    using namespace metal;

    struct Uniforms
    {
        float2 offset;
    };

    struct Vertex
    {
        float2 in_pos   [[attribute(0)]];
        float2 in_uv    [[attribute(1)]];
    };

    struct RasterizerData
    {
        float4 position [[position]];
        float2 uv       [[user(locn0)]];
    };

    vertex RasterizerData vertexShader(
      Vertex v [[stage_in]], 
      constant Uniforms& uniforms [[buffer(0)]])
    {
        RasterizerData out;

        out.position = float4(v.in_pos.xy + uniforms.offset, 0.0, 1.0);
        out.uv = v.in_uv;

        return out;
    }

    fragment float4 fragmentShader(RasterizerData in [[stage_in]], texture2d<float> tex [[texture(0)]], sampler texSmplr [[sampler(0)]])
    {
        return tex.sample(texSmplr, in.uv);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("offset", UniformType::Float2)],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub offset: (f32, f32),
    }
}
