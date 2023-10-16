use std::io::Write;

pub mod frame;
pub mod panel;

#[derive(Debug, Clone)]
pub struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

pub trait Glyph {
    fn write_to(&self, w: &mut dyn Write);
    fn area(&self) -> Rect;
}