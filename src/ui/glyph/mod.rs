use std::io::Write;

pub mod frame;
pub mod panel;
pub mod label;

#[derive(Debug, Clone)]
pub struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}
impl Rect {
    fn new() -> Rect {
        Rect { x: 0, y: 0, w: 0, h: 0 }
    }
}

#[derive(Debug, Clone)]
pub enum Requirement{
    Chars(u16),
    Max,
}

#[derive(Debug, Clone)]
pub struct Requirements {
    pub w: Requirement,
    pub h: Requirement,
}

pub trait Glyph {
    fn write_to(&self, w: &mut dyn Write);
    fn area(&self) -> Rect;
    fn resize(&mut self, width: u16, height: u16);
    fn handle_event(&mut self, r: crossterm::event::Event) -> bool;
    fn request(&mut self) -> Requirements;
    fn allocate(&mut self, allocation: Rect);
}
