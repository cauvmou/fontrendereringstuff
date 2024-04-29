use log::{info, trace};
use crate::{GlyphData, TEXTURE_SIZE};
use crate::renderer::GlyphVertex;

#[derive(Clone, Debug)]
pub struct GlyphMesh {
    pub glyph_id: ttf_parser::GlyphId,
    pub vertices: Vec<GlyphVertex>,
    pub indices: Vec<u16>,
    pub bounds: ttf_parser::Rect,
}

pub struct GlyphMeshBuilder {
    reverse_wind: bool,
    polygons: Vec<Vec<(f32, f32)>>,
    bezier_polygons: Vec<([(f32, f32); 3], bool)>,
}

impl GlyphMeshBuilder {
    pub fn new() -> Self {
        Self {
            reverse_wind: false,
            polygons: vec![],
            bezier_polygons: vec![],
        }
    }

    pub fn build(mut self, face: &ttf_parser::Face, glyph_id: ttf_parser::GlyphId) -> Option<GlyphMesh> {
        // Check winding direction
        self.reverse_wind = !face.tables().glyf.is_some();
        trace!("font winding order: {}", if self.reverse_wind { "clockwise" } else { "counter-clockwise" });
        trace!("    table glyf: {}", if face.tables().glyf.is_some() {"exists"} else {"absent"});
        trace!("    table cff: {}", if face.tables().cff.is_some() {"exists"} else {"absent"});
        trace!("    table cff2: {}", if face.tables().cff2.is_some() {"exists"} else {"absent"});
        let Some(bounds) = face.outline_glyph(glyph_id, &mut self) else {
            return None;
        };
        let (vertices, indices) = self.triangulate();
        Some(GlyphMesh {
            glyph_id,
            vertices,
            indices,
            bounds,
        })
    }

    pub fn triangulate(&self) -> (Vec<GlyphVertex>, Vec<u16>) {
        // check for holes
        let is_polygon_hole = self.polygons.iter().map(|points| {
            // Sum over edges
            is_ccw_wind(&points) ^ self.reverse_wind
        }).collect::<Vec<bool>>();

        // Group Polygons
        let mut polygon_with_holes: Vec<Vec<Vec<Vec<f32>>>> = vec![];
        for (index, points) in self.polygons.iter().enumerate() {
            if is_polygon_hole[index] {
                let element = polygon_with_holes.last_mut().unwrap();
                element.push(points.iter().map(|v| vec![v.0, v.1]).collect());
            } else {
                polygon_with_holes.push(vec![points.iter().map(|v| vec![v.0, v.1]).collect::<Vec<Vec<f32>>>()]);
            }
        }
        trace!("grouped {:?} meshes", polygon_with_holes.len());

        // triangulate
        let mut indices: Vec<u16> = vec![];
        let mut vertices: Vec<GlyphVertex> = vec![];
        for points in polygon_with_holes {
            // flatten
            let (points, holes, dimensions) = earcutr::flatten(&points);

            // Calculate indices
            indices.append(&mut earcutr::earcut(
                &points, &holes, dimensions,
            ).unwrap()
                .iter().map(|t| (vertices.len() + *t) as u16).collect());

            // Map point format
            let (even, odd): (Vec<(usize, &f32)>, Vec<(usize, &f32)>) = points.iter().enumerate().partition(|(index, _v)| index % 2 == 0);
            let points = even.iter().map(|v| *v.1).zip(odd.iter().map(|v| *v.1)).collect::<Vec<(f32, f32)>>();

            // Map to vertices
            vertices.append(&mut points.iter().map(|(x, y)| GlyphVertex {
                position: [*x, *y, 0.0], // Only temp
                uv: [0.0, 0.0],
                metadata: 0,
                color: [1.0, 1.0, 1.0],
            }).collect());
        }
        for (polygon, is_inverse) in &self.bezier_polygons {
            let index = vertices.len() as u16;
            let mut i = vec![index, index + 1, index + 2];
            indices.append(&mut i);
            vertices.append(&mut polygon.iter().enumerate().map(|(index, (x, y))| GlyphVertex {
                position: [*x, *y, 0.0], // Only temp
                uv: [[0.0, 0.0], [0.5, 0.0], [1.0, 1.0]][index],
                metadata: 0b10 | *is_inverse as i32,
                color: [1.0, 1.0, 1.0],
            }).collect());
        }
        trace!("finished triangulating");
        (vertices, indices)
    }
}

impl ttf_parser::OutlineBuilder for GlyphMeshBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.polygons.push(Vec::new());
        self.polygons.last_mut().unwrap().push((x, y))
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.polygons.last_mut().unwrap().push((x, y))
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let points = [*self.polygons.last().unwrap().last().unwrap(), (x1, y1), (x, y)];
        let is_inverse = is_ccw_wind(&points) ^ self.reverse_wind;
        self.bezier_polygons.push((points, is_inverse));
        if is_inverse {
            self.polygons.last_mut().unwrap().push((x1, y1));
        }
        self.polygons.last_mut().unwrap().push((x, y));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (ix, iy) = (x1 + (x2 - x1) / 2.0, y1 + (y2 - y1) / 2.0);
        let points = [*self.polygons.last().unwrap().last().unwrap(), (x1, y1), (ix, iy)];
        let is_inverse = is_ccw_wind(&points) ^ self.reverse_wind;
        self.bezier_polygons.push((points, is_inverse));
        if is_inverse {
            self.polygons.last_mut().unwrap().push((x1, y1));
        }
        self.polygons.last_mut().unwrap().push((ix, iy)); // Implied point by cubic bezier
        let points = [(ix, iy), (x2, y2), (x, y)];
        let is_inverse = is_ccw_wind(&points) ^ self.reverse_wind;
        self.bezier_polygons.push((points, is_inverse));
        if is_inverse {
            self.polygons.last_mut().unwrap().push((x2, y2));
        }
        self.polygons.last_mut().unwrap().push((x, y));
    }

    fn close(&mut self) {
        
    }
}

fn is_ccw_wind(vertices: &[(f32, f32)]) -> bool {
    let mut sum = 0.0;
    for index in 0..vertices.len() {
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        sum += current.0 * next.1 - next.0 * current.1;
    }
    sum >= 0.0
}

pub struct TextMesh {
    pub vertices: Vec<GlyphVertex>,
    pub indices: Vec<u16>,
}

pub struct TextMeshBuilder {
    mesh_data: Vec<(Option<GlyphMesh>, GlyphData)>,
    font_size: usize,
    position: (i32, i32)
}

impl TextMeshBuilder {
    pub fn new() -> Self {
        Self {
            mesh_data: vec![],
            font_size: 500,
            position: (100, 100),
        }
    }
    
    pub fn with_font_size(&mut self, font_size: usize) -> &mut Self {
        self.font_size = font_size;
        self
    }

    pub fn with_position(&mut self, x: i32, y: i32) -> &mut Self {
        self.position.0 = x;
        self.position.1 = y;
        self
    }

    pub fn add(&mut self, mesh: Option<GlyphMesh>, data: GlyphData) -> &mut Self {
        self.mesh_data.push((mesh, data));
        self
    }

    pub fn build(mut self, face: &ttf_parser::Face) -> TextMesh {
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
        trace!("constructed TextMesh with {} vertices and {} indices", vertices.len(), indices.len());
        TextMesh {
            vertices,
            indices,
        }
    }
}