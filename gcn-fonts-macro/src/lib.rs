extern crate proc_macro;

const I8_BLOCK_WIDTH: usize = 8;
const I8_BLOCK_HEIGHT: usize = 4;

use proc_macro::TokenStream;

use image::imageops::overlay;
use image::GrayImage;
use proc_macro_hack::proc_macro_hack;
use rusttype::gpu_cache::CacheBuilder;
use rusttype::Point;
use rusttype::{Font, Rect, Scale};
use syn::{
    parse::{Parse, ParseStream},
    LitFloat, LitInt, LitStr,
};

use std::fs;
use std::io::prelude::*;

mod kw {
    syn::custom_keyword!(path);
    syn::custom_keyword!(resolution);
    syn::custom_keyword!(size);
    syn::custom_punctuation!(Colon, :);
    syn::custom_punctuation!(Comma, ,);
    syn::custom_punctuation!(Star, *);
}

#[derive(Debug)]
struct Glyph {
    descender: f32,
    bounds: Rect<f32>,
}

struct Resolution {
    width: LitInt,
    height: LitInt,
}

impl Parse for Resolution {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _: kw::Colon = input.parse()?;
        let width: LitInt = input.parse()?;
        let _: kw::Star = input.parse()?;
        let height: LitInt = input.parse()?;
        let _: Option<kw::Comma> = input.parse()?;
        Ok(Resolution { width, height })
    }
}

// impl Synom for Resolution {
//     named!(parse -> Self, do_parse!(
//         custom_keyword!(resolution) >>
//         punct!(:) >>
//         width: syn!(LitInt) >>
//         punct!(*) >>
//         height: syn!(LitInt) >>
//         option!(punct!(,)) >>
//         (Resolution {
//             width,
//             height,
//         })
//     ));
// }

struct Size {
    size: LitFloat,
}

impl Parse for Size {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _: kw::Colon = input.parse()?;
        let size: LitFloat = input.parse()?;
        let _: Option<kw::Comma> = input.parse()?;
        Ok(Size { size })
    }
}

// impl Synom for Size {
//     named!(parse -> Self, do_parse!(
//         custom_keyword!(size) >>
//         punct!(:) >>
//         size: syn!(LitFloat) >>
//         option!(punct!(,)) >>
//         (Size {
//             size,
//         })
//     ));
// }

struct Params {
    path: LitStr, // 💯🔥😂👌
    resolution: Option<Resolution>,
    size: Option<Size>,
}

// impl Synom for Params {
//     named!(parse -> Self, do_parse!(
//         custom_keyword!(path) >>
//         punct!(:) >>
//         path: syn!(LitStr) >>
//         option!(punct!(,)) >>
//         resolution: option!(syn!(Resolution)) >>
//         size: option!(syn!(Size)) >>
//         (Params {
//             path,
//             resolution,
//             size,
//         })
//     ));
// }

impl Parse for Params {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _: kw::path = input.parse()?;
        let _: kw::Colon = input.parse()?;
        let path = input.parse()?;
        let _: Option<kw::Comma> = input.parse()?;
        let resolution_kw: Option<kw::resolution> = input.parse()?;
        let resolution = if let Some(_) = resolution_kw {
            Some(input.parse()?)
        } else {
            None
        };
        let size_kw: Option<kw::size> = input.parse()?;
        let size = if let Some(_) = size_kw {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Params {
            path,
            resolution,
            size,
        })
    }
}

#[proc_macro_hack]
pub fn include_font(input: TokenStream) -> TokenStream {
    let params: Params = syn::parse(input).unwrap();

    let (width, height) = params
        .resolution
        .map(|r| {
            (
                r.width.base10_parse().unwrap(),
                r.height.base10_parse().unwrap(),
            )
        })
        .unwrap_or((256, 256));
    let size = params
        .size
        .map(|s| s.size.base10_parse().unwrap())
        .unwrap_or(50.0);
    let scale = Scale::uniform(size);

    let font_data = fs::read(params.path.value()).unwrap();
    let font = Font::from_bytes(&font_data).expect("Error constructing Font");
    let mut atlas = GrayImage::new(width, height);

    let mut cache = CacheBuilder::default().dimensions(width, height).build();

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
                    .unwrap() //expect("Failed to get rect.")
                    .unwrap() //expect("Failed to unwrap TextureCoords")
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
