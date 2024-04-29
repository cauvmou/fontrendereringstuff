mod mesh;
mod renderer;
mod text;

use simple_logger::SimpleLogger;
use wgpu::util::{DeviceExt};
use std::borrow::BorrowMut;
use log::{debug, LevelFilter, trace};
use crate::mesh::{TextMesh};
use crate::renderer::GlyphVertex;
use crate::text::{Alignment, Span};

pub const TEXTURE_SIZE: (u32, u32) = (1920u32, 1080u32);
// const FONT_PATH: &'static str = "./fonts/NotoSansJP-Regular.ttf";
// const FONT_PATH: &'static str = "/usr/share/fonts/liberation/LiberationMono-Regular.ttf";
const FONT_PATH: &'static str = "/usr/share/fonts/gnu-free/FreeSans.otf";

#[derive(Copy, Clone, Debug)]
pub struct GlyphData {
    glyph_id: u32,
    x_advance: i32,
    y_advance: i32,
    x_offset: i32,
    y_offset: i32,
}

fn main() {
    SimpleLogger::new()/*.with_level(LevelFilter::Debug)*/.init().unwrap();

    // Load font
    let raw_font_data = std::fs::read(FONT_PATH).unwrap();
    let face = ttf_parser::Face::parse(&raw_font_data, 0).unwrap();

    let TextMesh { vertices, indices } = Span::new(
        &face,
        "Hello, World!",
        0,
        0)
        .with_font_size(24)
        .with_size(TEXTURE_SIZE.0 as usize, TEXTURE_SIZE.1 as usize)
        .with_h_align(Alignment::Middle)
        .with_v_align(Alignment::Middle)
        .generate_text_mesh();

    // Setup wgpu
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = {
        let mut all_adapters = instance.enumerate_adapters(wgpu::Backends::PRIMARY);
        let adapter_index = all_adapters.iter()
            .position(|adapter| adapter.features().contains(wgpu::Features::POLYGON_MODE_LINE)).unwrap();
        all_adapters.remove(adapter_index)
    };
    let (device, queue) = pollster::block_on(adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::default() | wgpu::Features::POLYGON_MODE_LINE,
            required_limits: Default::default(),
        }, None)
    ).unwrap();

    // Create texture to write to
    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: TEXTURE_SIZE.0,
            height: TEXTURE_SIZE.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
        ,
        label: None,
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
    };
    let texture = device.create_texture(&texture_desc);
    let texture_view = texture.create_view(&Default::default());

    // Create the output buffer
    let u32_size = std::mem::size_of::<u32>() as u32;
    let output_buffer_size = ((u32_size * TEXTURE_SIZE.0) * TEXTURE_SIZE.1) as wgpu::BufferAddress;
    let output_buffer_desc = wgpu::BufferDescriptor {
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST
            // this tells wpgu that we want to read this buffer from the cpu
            | wgpu::BufferUsages::MAP_READ,
        label: None,
        mapped_at_creation: false,
    };
    let output_buffer = device.create_buffer(&output_buffer_desc);

    // Compile and create shader modules
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(include_str!("shader/glyph.wgsl").into()),
    });

    // Create vertex buffer
    let vertex_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }
    );

    // Create index buffer
    let index_buffer = device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }
    );

    // Create render pipeline
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[
                GlyphVertex::desc(),
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: texture_desc.format,
                write_mask: wgpu::ColorWrites::ALL,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None, //Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: true,
        },
        multiview: None,
    });

    // SMAA
    let mut smaa_target = smaa::SmaaTarget::new(
        &device,
        &queue,
        TEXTURE_SIZE.0,
        TEXTURE_SIZE.1,
        texture.format(),
        smaa::SmaaMode::Smaa1X
    );

    let smaa_frame = smaa_target.start_frame(&device, &queue, &texture_view);

    // Render encoder and pass
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: None,
    });

    {
        let render_pass_desc = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &smaa_frame,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };
        let mut render_pass = encoder.begin_render_pass(&render_pass_desc);

        render_pass.set_pipeline(&render_pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..(indices.len() as u32), 0, 0..1);
    }

    queue.submit(Some(encoder.finish()));
    smaa_frame.resolve();

    // Copy texture to output buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: None,
    });

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(u32_size * TEXTURE_SIZE.0),
                rows_per_image: Some(TEXTURE_SIZE.1),
            },
        },
        texture_desc.size,
    );

    queue.submit(Some(encoder.finish()));

    // Write image to disk and unmap output buffer
    {
        let buffer_slice = output_buffer.slice(..);

        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();

        use image::{ImageBuffer, Rgba};
        let buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(TEXTURE_SIZE.0, TEXTURE_SIZE.1, data).unwrap();
        buffer.save("./image.png").unwrap();
    }
    output_buffer.unmap();
}