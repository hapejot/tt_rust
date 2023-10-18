use crossterm::{cursor::MoveTo, style::Print, QueueableCommand};
use tracing::info;

use super::{
    AppError::{self, NotRelevant},
    AppRequest::{self, SetValue},
    AppResult::{self, Redraw},
    Glyph, Rect,
};

pub struct Label {
    area: Rect,
    txt: String,
}

impl Label {
    pub fn new(txt: String) -> Self {
        Self {
            area: Rect::new(),
            txt,
        }
    }
}

impl Glyph for Label {
    fn write_to(&self, w: &mut dyn std::io::Write) {
        w.queue(MoveTo(self.area.x, self.area.y)).unwrap();
        w.queue(Print(self.txt.clone())).unwrap();
    }

    fn area(&self) -> Rect {
        todo!()
    }

    fn resize(&mut self, width: u16, height: u16) {
        todo!()
    }

    fn handle_term_event(&mut self, event: crossterm::event::Event) -> bool {
        match event {
            _ => false,
        }
    }

    fn request(&mut self) -> super::Requirements {
        todo!()
    }

    fn allocate(&mut self, allocation: Rect) {
        self.area = allocation;
        info!("allocate label to {:?}", &self.area);
    }

    fn handle_app_request(&mut self, req: &AppRequest) -> Result<AppResult, AppError> {
        match req {
            SetValue(v) => {
                self.txt = v.clone();
                Ok(Redraw)
            }
            _ => Err(NotRelevant),
        }
    }
}
