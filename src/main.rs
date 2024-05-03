mod mesh;
mod renderer;
mod text;

use simple_logger::SimpleLogger;
use wgpu::util::{DeviceExt};
use std::borrow::BorrowMut;
use log::{debug, LevelFilter, trace};
use crate::mesh::{TextMesh};
use crate::renderer::{AAMode, GlyphVertex, TextureRenderer};
use crate::text::{Alignment, FontSize, Span};
use image::{ImageBuffer, Rgba};

pub const TEXTURE_SIZE: (u32, u32) = (1920u32, 1920u32);
// const FONT_PATH: &'static str = "./fonts/NotoSansJP-Regular.ttf";
// const FONT_PATH: &'static str = "/usr/share/fonts/liberation/LiberationMono-Regular.ttf";
// const FONT_PATH: &'static str = "/usr/share/fonts/gnu-free/FreeSans.otf";
const FONT_PATH: &'static str = "/usr/share/fonts/TTF/Iosevka-Regular.ttf";

#[derive(Copy, Clone, Debug)]
pub struct GlyphData {
    glyph_id: u32,
    x_advance: i32,
    y_advance: i32,
    x_offset: i32,
    y_offset: i32,
}

fn main() {
    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();

    // Load font
    let raw_font_data = std::fs::read(FONT_PATH).unwrap();
    let face = ttf_parser::Face::parse(&raw_font_data, 0).unwrap();
    let mut renderer = TextureRenderer::new(TEXTURE_SIZE.0, TEXTURE_SIZE.1, AAMode::MSAAx8);

    renderer.add_span(Span::new(
        &face,
        "SimpleLogger::new()/*.with_level(LevelFilter::Debug)*/.init().unwrap();",
        0,
        0)
        .with_font_size(FontSize::Pt(8))
        .with_size(TEXTURE_SIZE.0 as usize, TEXTURE_SIZE.1 as usize / 3)
        .with_h_align(Alignment::Middle)
        .with_v_align(Alignment::Middle)
        .with_color([0.0, 0.0, 0.0, 1.0])
    )
        .add_span(Span::new(
            &face,
            "SimpleLogger::new()/*.with_level(LevelFilter::Debug)*/.init().unwrap();",
            0,
            (TEXTURE_SIZE.1 / 3) as i32)
            .with_font_size(FontSize::Pt(24))
            .with_size(TEXTURE_SIZE.0 as usize, TEXTURE_SIZE.1 as usize / 3)
            .with_h_align(Alignment::Middle)
            .with_v_align(Alignment::Middle)
            .with_color([0.0, 1.0, 0.0, 1.0])
        )
        .add_span(Span::new(
            &face,
            "SimpleLogger::new()/*.with_level(LevelFilter::Debug)*/.init().unwrap();",
            0,
            (2 * TEXTURE_SIZE.1 / 3) as i32)
            .with_font_size(FontSize::Pt(36))
            .with_size(TEXTURE_SIZE.0 as usize, TEXTURE_SIZE.1 as usize / 3)
            .with_h_align(Alignment::Middle)
            .with_v_align(Alignment::Middle)
            .with_color([0.0, 0.0, 1.0, 1.0])
        );

    let image = renderer.render();
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(TEXTURE_SIZE.0, TEXTURE_SIZE.1, image).unwrap();
    buffer.save("./image.png").unwrap();
}