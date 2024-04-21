use log::info;
use crate::{GlyphData, TEXTURE_SIZE};
use crate::mesh::GlyphMesh;
use crate::renderer::GlyphVertex;

pub mod shape;

pub struct TextMesh {
    pub vertices: Vec<GlyphVertex>,
    pub indices: Vec<u16>,
}

pub struct TextMeshBuilder {
    mesh_data: Vec<(Option<GlyphMesh>, GlyphData)>,
    font_size: usize,
    position: (usize, usize)
}

impl TextMeshBuilder {
    pub fn new() -> Self {
        Self {
            mesh_data: vec![],
            font_size: 200,
            position: (100, 200),
        }
    }

    pub fn add(&mut self, mesh: Option<GlyphMesh>, data: GlyphData) {
        self.mesh_data.push((mesh, data))
    }

    pub fn build(mut self, face: &ttf_parser::Face) -> TextMesh {
        let aspect_factor = TEXTURE_SIZE.0 as f32 / TEXTURE_SIZE.1 as f32;
        let size_factor = 1.0 / face.height() as f32;
        let mut vertices: Vec<GlyphVertex> = vec![];
        let mut indices: Vec<u16> = vec![];
        let mut cursor = (0.0, 0.0);
        for (mesh, data) in &mut self.mesh_data {
            if let Some(mesh) = mesh {
                indices.append(&mut mesh.indices.iter().map(|i| *i + (vertices.len() as u16)).collect());
                vertices.append(&mut mesh.vertices.iter_mut().map(|v| {
                    v.position[0] += cursor.0;
                    v.position[1] += cursor.1;

                    v.position[0] = v.position[0] * size_factor * self.font_size as f32 * 1.254;
                    v.position[1] = v.position[1] * size_factor * self.font_size as f32 * 1.254;
                    v.position[0] = v.position[0] / TEXTURE_SIZE.0 as f32 * 2.0 - 1.0;
                    v.position[1] = v.position[1] / TEXTURE_SIZE.1 as f32 * 2.0 - 1.0;
                    v.position[0] += (self.position.0 as f32 / TEXTURE_SIZE.0 as f32) * 2.0;
                    v.position[1] += (self.position.1 as f32 / TEXTURE_SIZE.1 as f32) * 2.0;
                    *v
                }).collect());
            }
            cursor.0 += data.x_advance as f32;
            cursor.1 += data.y_advance as f32;
        }
        info!("Constructed TextMesh with {} vertices", vertices.len());
        TextMesh {
            vertices,
            indices,
        }
    }
}