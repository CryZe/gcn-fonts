#![no_std]

use core::cell::UnsafeCell;
use gcn::gx;
use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack]
pub use gcn_fonts_macro::include_font;

pub mod prelude;

use prelude::Glyph;

pub struct DecodedGlyph<'a> {
    glyph: &'a Glyph,
    advance_x: f32,
    width: f32,
    height: f32,
}

impl DecodedGlyph<'_> {
    pub fn position(&self, x: f32, y: f32) -> PositionedGlyph {
        let y = y + self.glyph.descender;
        let ty = y - self.height;
        let by = y;
        let rx = x + self.width;
        let lx = x;
        let vertices = [[lx, ty], [rx, ty], [rx, by], [lx, by]];

        let next_x = x + self.advance_x;

        let bounds = &self.glyph.bounds;
        let ty = bounds.min.y;
        let by = bounds.max.y;
        let rx = bounds.max.x;
        let lx = bounds.min.x;
        let tex_coords = [[lx, ty], [rx, ty], [rx, by], [lx, by]];

        PositionedGlyph {
            vertices,
            tex_coords,
            next_x,
        }
    }
}

pub struct PositionedGlyph {
    vertices: [[f32; 2]; 4],
    tex_coords: [[f32; 2]; 4],
    next_x: f32,
}

impl PositionedGlyph {
    pub fn render(&self, color: u32) {
        unsafe {
            gx::begin(gx::QUADS, gx::VTXFMT0, 4);
            for (position, tex_coord) in self.vertices.iter().zip(&self.tex_coords) {
                gx::submit_f32s(position);
                gx::submit_u32(color);
                gx::submit_f32s(tex_coord);
            }
            gx::end();
        }
    }
}

pub struct UploadedFont {
    texture: UnsafeCell<gx::TexObj>,
    pub font: Font,
}

impl UploadedFont {
    pub fn lookup_glyph(&self, c: char) -> Option<DecodedGlyph<'_>> {
        let c = (c as usize).checked_sub(0x21)?;
        let glyph = self.font.glyphs.get(c)?;
        let bounds = &glyph.bounds;
        let width = (bounds.max.x - bounds.min.x) * self.font.width;
        let height = (bounds.max.y - bounds.min.y) * self.font.height;
        let advance_x = width; // + 0.5;

        Some(DecodedGlyph {
            glyph,
            width,
            height,
            advance_x,
        })
    }

    /// Returns the next x coordinate
    pub fn render_char(&self, c: char, x: f32, y: f32, color: u32) -> f32 {
        if let Some(glyph) = self.lookup_glyph(c) {
            let glyph = glyph.position(x, y);
            glyph.render(color);
            glyph.next_x
        } else {
            x + self.font.space_advance
        }
    }

    pub fn render_chars<C>(&self, chars: C, mut x: f32, y: f32, color: u32)
    where
        C: IntoIterator<Item = char>,
    {
        for c in chars {
            x = self.render_char(c, x, y, color);
        }
    }

    pub fn measure_text_width<C>(&self, chars: C) -> f32
    where
        C: IntoIterator<Item = char>,
    {
        chars.into_iter().map(|c| self.measure_char(c)).sum()
    }

    pub fn measure_char(&self, c: char) -> f32 {
        if let Some(glyph) = self.lookup_glyph(c) {
            glyph.advance_x
        } else {
            self.font.space_advance
        }
    }

    pub fn render_chars_centered<C>(&self, chars: C, x: f32, y: f32, color: u32)
    where
        C: IntoIterator<Item = char>,
        C::IntoIter: Clone,
    {
        let iter = chars.into_iter();
        let width = self.measure_text_width(iter.clone());
        let x = x - 0.5 * width;
        self.render_chars(iter, x, y, color);
    }

    pub fn stop_rendering(&self) {
        unsafe {
            gx::clear_vtx_desc();
            gx::set_vtx_desc(gx::VA_POS as u8, gx::DIRECT);
            gx::set_vtx_desc(gx::VA_CLR0 as u8, gx::DIRECT);

            gx::set_tev_order(
                gx::TEVSTAGE0,
                gx::TEXCOORD0,
                gx::TEXMAP_DISABLE,
                gx::COLOR0A0,
            );
        }
    }

    pub fn setup_rendering(&self) {
        unsafe {
            gx::set_blend_mode(
                gx::BM_BLEND,
                gx::BL_SRCALPHA,
                gx::BL_INVSRCALPHA,
                gx::LO_SET,
            );
            gx::clear_vtx_desc();
            gx::set_vtx_desc(gx::VA_POS as u8, gx::DIRECT);
            gx::set_vtx_desc(gx::VA_CLR0 as u8, gx::DIRECT);
            gx::set_vtx_desc(gx::VA_TEX0 as u8, gx::DIRECT);

            gx::set_vtx_attr_fmt(gx::VTXFMT0, gx::VA_POS, gx::POS_XY, gx::F32, 0);
            gx::set_vtx_attr_fmt(gx::VTXFMT0, gx::VA_CLR0, gx::CLR_RGBA, gx::RGBA8, 0);
            gx::set_vtx_attr_fmt(gx::VTXFMT0, gx::VA_TEX0, gx::TEX_ST, gx::F32, 0);

            gx::set_num_tex_gens(1);
            gx::set_tex_coord_gen(
                gx::TEXCOORD0 as u16,
                gx::TG_MTX2X4,
                gx::TG_TEX0,
                gx::IDENTITY,
            );

            gx::set_tev_color_in(
                gx::TEVSTAGE0,
                gx::CC_ZERO,
                gx::CC_ZERO,
                gx::CC_ZERO,
                gx::CC_RASC,
            );
            gx::set_tev_alpha_in(
                gx::TEVSTAGE0,
                gx::CA_ZERO,
                gx::CA_RASA,
                gx::CA_TEXA,
                gx::CA_ZERO,
            );
            gx::set_tev_color_op(
                gx::TEVSTAGE0,
                gx::TEV_ADD,
                gx::TB_ZERO,
                gx::CS_SCALE_1,
                gx::TRUE,
                gx::TEVPREV,
            );
            gx::set_tev_alpha_op(
                gx::TEVSTAGE0,
                gx::TEV_ADD,
                gx::TB_ZERO,
                gx::CS_SCALE_1,
                gx::TRUE,
                gx::TEVPREV,
            );
            gx::set_tev_order(gx::TEVSTAGE0, gx::TEXCOORD0, gx::TEXMAP0, gx::COLOR0A0);
            gx::load_tex_obj(self.texture.get(), gx::TEXMAP0 as u8);
        }
    }
}

#[derive(Copy, Clone)]
pub struct Font {
    pub width: f32,
    pub height: f32,
    pub size: f32,
    pub space_advance: f32,
    pub data: &'static [u8],
    pub glyphs: &'static [Glyph],
}

impl Font {
    pub fn upload(&self) -> UploadedFont {
        let mut tex_obj = gx::TexObj::default();
        unsafe {
            gx::init_tex_obj(
                &mut tex_obj,
                self.data.as_ptr(),
                self.width as u16,
                self.height as u16,
                gx::TF_I8,
                gx::CLAMP,
                gx::CLAMP,
                gx::FALSE,
            );
        }
        UploadedFont {
            texture: UnsafeCell::new(tex_obj),
            font: *self,
        }
    }
}
