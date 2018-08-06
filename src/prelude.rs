use core::cell::UnsafeCell;
use gcn::gx;
use print_char;

pub use gcn_fonts_macro::include_font;

#[repr(align(32))]
pub struct AlignedData<T>(pub T);

#[derive(Copy, Clone)]
pub struct Font {
    pub width: f32,
    pub height: f32,
    pub size: f32,
    pub space_advance: f32,
    pub data: &'static [u8],
    pub glyphs: &'static [Glyph],
}

pub struct UploadedFont {
    pub texture: UnsafeCell<gx::TexObj>,
    pub font: Font,
}

#[derive(Debug)]
pub struct Glyph {
    pub descender: f32,
    pub bounds: Rect,
}

#[derive(Debug)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

#[derive(Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
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

impl UploadedFont {
    pub fn print(&self, text: &str, mut x: f32, y: f32, top_color: u32, bottom_color: u32) {
        text.chars().for_each(|c| {
            let adv_x = print_char(self, c, x, y, top_color, bottom_color);
            x += adv_x + 0.5;
        });
    }
}
