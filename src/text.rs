use std::borrow::BorrowMut;
use crate::{GlyphData};
use crate::mesh::{GlyphMeshBuilder, TextMesh, TextMeshBuilder};

#[derive(Copy, Clone, Debug, Default)]
pub enum Alignment {
    #[default]
    Start,
    Middle,
    End,
}

#[derive(Copy, Clone, Debug)]
pub enum FontSize {
    Px(usize),
    Pt(usize)
}

impl Into<f32> for FontSize {
    fn into(self) -> f32 {
        match self {
            FontSize::Px(x) => {x as f32}
            FontSize::Pt(x) => {(x as f32 / 72.0) * 150.0 }
        }
    }
}

impl Into<i32> for FontSize {
    fn into(self) -> i32 {
        match self {
            FontSize::Px(x) => {x as i32}
            FontSize::Pt(x) => {((x as f32 / 72.0) * 150.0).round() as i32 }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Span<'s> {
    text: &'s str,
    font_face: &'s ttf_parser::Face<'s>,
    position: (i32, i32),
    font_size: FontSize,
    size: Option<(usize, usize)>,
    v_align: Alignment,
    h_align: Alignment,
    color: [f32; 4]
}

impl<'s> Span<'s> {
    pub fn new(font_face: &'s ttf_parser::Face<'s>, text: &'s str, x: i32, y: i32) -> Self {
        Self {
            text,
            font_face,
            position: (x, y),
            font_size: FontSize::Pt(12),
            size: None,
            v_align: Alignment::Start,
            h_align: Alignment::Start,
            color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn with_size(mut self, width: usize, height: usize) -> Self {
        self.size = Some((width, height));
        self
    }
    
    pub fn with_font_size(mut self, font_size: FontSize) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn with_v_align(mut self, v_align: Alignment) -> Self {
        self.v_align = v_align;
        self
    }

    pub fn with_h_align(mut self, h_align: Alignment) -> Self {
        self.h_align = h_align;
        self
    }
    
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }
    
    pub fn get_color(&self) -> [f32; 4] {
        self.color
    }

    pub fn generate_text_mesh(&self, color_index: u32) -> TextMesh {
        let glyph_data = self.shape_glyph_data();
        let mut text_mesh_builder = TextMeshBuilder::new();
        let mut width = 0.0;
        for data in glyph_data {
            width += data.x_advance as f32;
            let mesh = GlyphMeshBuilder::new().build(&self.font_face, ttf_parser::GlyphId(data.glyph_id as u16));
            text_mesh_builder.add(mesh, data);
        }
        // Align text
        width = width / self.font_face.height() as f32 * <FontSize as Into<f32>>::into(self.font_size) * 1.254; // Convert width to pixels
        let mut text_position: (i32, i32) = self.position;
        if let Some(size) = self.size {
            match self.h_align {
                Alignment::Start => {}
                Alignment::Middle => {
                    text_position.0 += size.0 as i32 / 2;
                    text_position.0 -= width as i32 / 2;
                }
                Alignment::End => {
                    text_position.0 += size.0 as i32;
                    text_position.0 -= width as i32;
                }
            }
            match self.v_align {
                Alignment::Start => {}
                Alignment::Middle => {
                    text_position.1 += size.1 as i32 / 2;
                    text_position.1 -= <FontSize as Into<i32>>::into(self.font_size) / 2;
                }
                Alignment::End => {
                    text_position.1 += size.1 as i32;
                    text_position.1 -= <FontSize as Into<i32>>::into(self.font_size);
                }
            }
        }
        text_mesh_builder.with_position(text_position.0, text_position.1);
        text_mesh_builder.with_font_size(self.font_size);
        text_mesh_builder.build(self.font_face, color_index)
    }

    fn shape_glyph_data(&self) -> Vec<GlyphData> {
        let mut glyph_data: Vec<GlyphData> = Vec::new();
        {
            let mut hb_buffer = harfbuzz::Buffer::with(self.text);
            hb_buffer.guess_segment_properties();
            let hb_buffer = hb_buffer.into_raw();
            let hb_blob = harfbuzz::Blob::new_read_only(&self.font_face.raw_face().data);
            let hb_face = unsafe { harfbuzz::sys::hb_face_create(hb_blob.as_raw(), 0) };
            let hb_font = unsafe { harfbuzz::sys::hb_font_create(hb_face) };
            unsafe { harfbuzz::sys::hb_shape(hb_font, hb_buffer, std::ptr::null(), 0) };
            let mut hb_glyph_count: u32 = 0;
            let hb_glyph_infos = unsafe { harfbuzz::sys::hb_buffer_get_glyph_infos(hb_buffer, hb_glyph_count.borrow_mut() as *mut u32) };
            let hb_glyph_positions = unsafe { harfbuzz::sys::hb_buffer_get_glyph_positions(hb_buffer, hb_glyph_count.borrow_mut() as *mut u32) };
            for index in 0..(hb_glyph_count as usize) {
                unsafe {
                    let hb_glyph_info = hb_glyph_infos.add(index);
                    let hb_glyph_position = hb_glyph_positions.add(index);
                    glyph_data.push(GlyphData {
                        glyph_id: (*hb_glyph_info).codepoint as u32,
                        x_advance: (*hb_glyph_position).x_advance as i32,
                        y_advance: (*hb_glyph_position).y_advance as i32,
                        x_offset: (*hb_glyph_position).x_offset as i32,
                        y_offset: (*hb_glyph_position).y_offset as i32,
                    })
                }
            }
        }
        glyph_data
    }
}