use log::info;
use wgpu::util::DeviceExt;
use crate::mesh::TextMesh;
use crate::text::Span;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub metadata: i32,
    pub color_index: u32
}

impl GlyphVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Sint32, 3 => Uint32];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq)]
pub enum AAMode {
    #[default]
    Disabled,
    MSAAx2,
    MSAAx4,
    MSAAx8,
}

impl AAMode {
    pub fn to_sample_count(&self) -> u32 {
        match self {
            AAMode::Disabled => 1,
            AAMode::MSAAx2 => 2,
            AAMode::MSAAx4 => 4,
            AAMode::MSAAx8 => 8,
        }
    }

    pub fn needs_extra_feature(&self) -> bool {
        match self {
            AAMode::Disabled => false,
            AAMode::MSAAx2 => true,
            AAMode::MSAAx4 => false,
            AAMode::MSAAx8 => true,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SubpixelOffsetInstanceRaw {
    offset: f32,
}

impl SubpixelOffsetInstanceRaw {
    const ATTRIBUTES: [wgpu::VertexAttribute; 1] =
        wgpu::vertex_attr_array![4 => Float32];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SubpixelOffsetInstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Holds state for the render
pub struct TextureRenderer<'r> {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_texture: wgpu::Texture,
    render_texture_view: wgpu::TextureView,
    output_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    color_bind_group_layout: wgpu::BindGroupLayout,
    spans: Vec<Span<'r>>,
    aa_mode: AAMode
}

impl<'r> TextureRenderer<'r> {
    pub fn new(width: u32, height: u32, mode: AAMode) -> Self {
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
                required_features: wgpu::Features::default() |
                    wgpu::Features::POLYGON_MODE_LINE |
                    wgpu::Features::POLYGON_MODE_POINT |
                    if mode.needs_extra_feature() {
                        wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    } else { wgpu::Features::empty() },
                required_limits: Default::default(),
            }, None)
        ).unwrap();

        // Create texture to write to
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
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

        // Create color storage layout
        let color_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("color_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: true,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }
                ],
            }
        );

        // Create the output buffer
        let u32_size = std::mem::size_of::<u32>() as u32;
        let output_buffer_size = ((u32_size * width) * height) as wgpu::BufferAddress;
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

        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&color_bind_group_layout],
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
                    SubpixelOffsetInstanceRaw::desc(),
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
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: mode.to_sample_count(),
                mask: !0,
                alpha_to_coverage_enabled: true,
            },
            multiview: None,
        });

        Self {
            instance,
            adapter,
            device,
            queue,
            render_texture: texture,
            render_texture_view: texture_view,
            output_buffer,
            pipeline: render_pipeline,
            color_bind_group_layout,
            spans: vec![],
            aa_mode: mode,
        }
    }

    pub fn add_span(&mut self, mesh: Span<'r>) -> &mut Self {
        self.spans.push(mesh);
        self
    }

    /// Returns raw image data in RgbaU8 format
    pub fn render(self) -> Vec<u8> {
        let mut all_vertices = vec![];
        let mut all_indices = vec![];
        let mut all_colors = vec![];
        for span in self.spans {
            if !all_colors.contains(&span.get_color()) {
                all_colors.push(span.get_color())
            }
            let TextMesh { mut vertices, indices } = span.generate_text_mesh(
                all_colors.iter().position(|c| *c == span.get_color()).unwrap_or(0) as u32
            );
            let last_index = all_vertices.len() as u16;
            all_indices.append(&mut indices.iter().map(|i| *i + last_index).collect());
            all_vertices.append(&mut vertices);
        }

        // Create vertex buffer
        let vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&all_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        // Create index buffer
        let index_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&all_indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        // Create color uniform
        let color_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Color Buffer"),
                contents: bytemuck::cast_slice(&all_colors),
                usage: wgpu::BufferUsages::STORAGE,
            }
        );

        let color_buffer_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("color_buffer_group"),
            layout: &self.color_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: color_buffer.as_entire_binding(),
                }
            ],
        });

        // MSAA
        // Create texture to write to
        let msaa_texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: self.render_texture.width(),
                height: self.render_texture.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: self.aa_mode.to_sample_count(),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
            ,
            label: None,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        };
        let msaa_texture = self.device.create_texture(&msaa_texture_desc);
        let msaa_texture_view = msaa_texture.create_view(&Default::default());

        // Create subpixel offset instances
        let third_pixel: f32 = (1.0 / self.render_texture.width() as f32);
        info!("pixel/3: {third_pixel}");
        let subpixel_instance_data = vec![
            SubpixelOffsetInstanceRaw { offset: -third_pixel },
            SubpixelOffsetInstanceRaw { offset: third_pixel },
            SubpixelOffsetInstanceRaw { offset: 0.0 },
        ];
        let subpixel_instance_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Subpixel Instance Buffer"),
                contents: bytemuck::cast_slice(&subpixel_instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        // Render encoder and pass
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });

        {
            let render_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: if self.aa_mode != AAMode::Disabled { &msaa_texture_view } else { &self.render_texture_view },
                        resolve_target: if self.aa_mode != AAMode::Disabled { Some(&self.render_texture_view) } else { None },
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
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

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &color_buffer_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, subpixel_instance_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..(all_indices.len() as u32), 0, 0..subpixel_instance_data.len() as _);
        }

        self.queue.submit(Some(encoder.finish()));

        // Copy texture to output buffer
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: None,
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &self.output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(std::mem::size_of::<u32>() as u32 * self.render_texture.width()),
                    rows_per_image: Some(self.render_texture.height()),
                },
            },
            self.render_texture.size(),
        );

        self.queue.submit(Some(encoder.finish()));

        // Save image and unmap output buffer
        let mut data = vec![];
        {
            let buffer_slice = self.output_buffer.slice(..);

            let (tx, rx) = std::sync::mpsc::channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            self.device.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();

            data = buffer_slice.get_mapped_range().to_vec();
        }
        self.output_buffer.unmap();
        data
    }
}