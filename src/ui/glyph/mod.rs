use std::io::Write;

pub mod frame;
pub mod input;
pub mod label;
pub mod panel;

#[derive(Debug, Clone)]
pub struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}
impl Rect {
    fn new() -> Rect {
        Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        }
    }

    fn contains(&self, x: u16, y: u16) -> bool {
        self.x <= x && self.y <= y && x < (self.x + self.w) && y < (self.y + self.h)
    }
}

#[derive(Debug, Clone)]
pub enum Requirement {
    Chars(u16),
    Max,
}

#[derive(Debug, Clone)]
pub struct Requirements {
    pub w: Requirement,
    pub h: Requirement,
}

#[derive(Debug, Clone)]
pub enum AppRequest {
    /// Dummy request to see what happens
    None,
    /// setting a value for a named element
    SetValue { name: String, value: String },
    /// getting a value for a named element somewhere in the tree
    GetValue(String),
    /// go to the next input field given the current cursor position
    NextInput(u16, u16),
    /// collect all values for named labels
    CollectAllValues,
}

#[derive(Debug, Clone)]
pub enum AppResult {
    StringValue(String),
    Redraw,
    InputEnabled,
    NewCursorPosition(u16, u16),
    Values(Vec<(String,String)>),
}

#[derive(Debug, Clone)]
pub enum AppError {
    NotRelevant,
    InvalidRequest,
}

pub type AppResponse = Result<AppResult, AppError>;

pub trait Glyph {
    fn hit(&mut self, x: u16, y: u16) -> AppResponse;
    fn write_to(&self, w: &mut dyn Write);
    fn area(&self) -> Rect;
    fn resize(&mut self, width: u16, height: u16);
    fn handle_term_event(&mut self, r: crossterm::event::Event) -> AppResponse;
    fn handle_app_request(&mut self, req: &AppRequest) -> AppResponse;
    fn request(&mut self) -> Requirements;
    fn allocate(&mut self, allocation: Rect);
    fn allocated(&self) -> bool;
}
