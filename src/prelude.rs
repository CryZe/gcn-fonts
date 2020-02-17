pub use crate::{Font, UploadedFont};

#[repr(align(32))]
pub struct AlignedData<T>(pub T);

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
