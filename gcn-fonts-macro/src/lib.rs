extern crate proc_macro;
#[macro_use]
extern crate syn;
extern crate image;
extern crate rusttype;

const I8_BLOCK_WIDTH: usize = 8;
const I8_BLOCK_HEIGHT: usize = 4;

use proc_macro::TokenStream;

use image::imageops::overlay;
use image::GrayImage;
use rusttype::gpu_cache::CacheBuilder;
use rusttype::Point;
use rusttype::{Font, Rect, Scale};
use syn::{synom::Synom, LitFloat, LitInt, LitStr};

use std::fs;
use std::io::prelude::*;

#[derive(Debug)]
struct Glyph {
    descender: f32,
    bounds: Rect<f32>,
}

struct Resolution {
    width: LitInt,
    height: LitInt,
}

impl Synom for Resolution {
    named!(parse -> Self, do_parse!(
        custom_keyword!(resolution) >>
        punct!(:) >>
        width: syn!(LitInt) >>
        punct!(*) >>
        height: syn!(LitInt) >>
        option!(punct!(,)) >>
        (Resolution {
            width,
            height,
        })
    ));
}

struct Size {
    size: LitFloat,
}

impl Synom for Size {
    named!(parse -> Self, do_parse!(
        custom_keyword!(size) >>
        punct!(:) >>
        size: syn!(LitFloat) >>
        option!(punct!(,)) >>
        (Size {
            size,
        })
    ));
}

struct Params {
    path: LitStr, // 💯🔥😂👌
    resolution: Option<Resolution>,
    size: Option<Size>,
}

impl Synom for Params {
    named!(parse -> Self, do_parse!(
        custom_keyword!(path) >>
        punct!(:) >>
        path: syn!(LitStr) >>
        option!(punct!(,)) >>
        resolution: option!(syn!(Resolution)) >>
        size: option!(syn!(Size)) >>
        (Params {
            path,
            resolution,
            size,
        })
    ));
}

#[proc_macro]
pub fn include_font(input: TokenStream) -> TokenStream {
    let params: Params = syn::parse(input).unwrap();

    let (width, height) = params
        .resolution
        .map(|r| (r.width.value() as u32, r.height.value() as u32))
        .unwrap_or((256, 256));
    let size = params.size.map(|s| s.size.value() as f32).unwrap_or(50.0);
    let scale = Scale::uniform(size);

    let font_data = fs::read(params.path.value()).unwrap();
    let font = Font::from_bytes(&font_data).expect("Error constructing Font");
    let mut atlas = GrayImage::new(width, height);

    let mut cache = CacheBuilder {
        width,
        height,
        ..CacheBuilder::default()
    }.build();

    let space_advance = font.glyph(' ').scaled(scale).h_metrics().advance_width;

    let glyphs = font
        .glyphs_for((0x21..=0x7E).map(|i: u8| i as char))
        .map(|g| g.scaled(scale).positioned(Point { x: 0.0, y: 0.0 }))
        .collect::<Vec<_>>();

    for glyph in &glyphs {
        cache.queue_glyph(0, glyph.clone());
    }

    cache
        .cache_queued(|rect, data| {
            let glyph = GrayImage::from_raw(rect.width(), rect.height(), data.to_vec())
                .expect("Bad GrayImage");
            overlay(&mut atlas, &glyph, rect.min.x, rect.min.y);
        })
        .expect("cache queue");

    let rects = glyphs
        .iter()
        .map(|glyph| {
            Glyph {
                descender: glyph.pixel_bounding_box().unwrap().max.y as f32,
                bounds: cache
                .rect_for(0, glyph)
                .unwrap()//expect("Failed to get rect.")
                .unwrap()//expect("Failed to unwrap TextureCoords")
                .0,
            }
        })
        .collect::<Vec<_>>();

    let mut buffer = Vec::with_capacity(width as usize * height as usize);

    {
        for row in 0..(height as usize / I8_BLOCK_HEIGHT) {
            let row_y = row * I8_BLOCK_HEIGHT;
            for column in 0..(width as usize / I8_BLOCK_WIDTH) {
                let column_x = column * I8_BLOCK_WIDTH;
                for y in 0..I8_BLOCK_HEIGHT {
                    let y = row_y + y;
                    let x = column_x;
                    let pixel_index = y * width as usize + x;
                    let src = &(*atlas)[pixel_index..][..I8_BLOCK_WIDTH];
                    buffer.write_all(src).unwrap();
                }
            }
        }
    }

    let tokens = format!(
        r#"Font {{
        width: {}.0,
        height: {}.0,
        size: {:?},
        space_advance: {:?},
        data: {{
            static DATA: AlignedData<[u8; {} * {}]> = AlignedData({:?});
            &DATA.0
        }},
        glyphs: &{:?},
    }}"#,
        width, height, size, space_advance, width, height, buffer, rects
    );

    tokens.parse().unwrap()
}
