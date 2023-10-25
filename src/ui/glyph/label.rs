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
    name: String,
    txt: String,
}

impl Label {
    pub fn new(name: String, txt: String) -> Self {
        Self {
            area: Rect::new(),
            name,
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

    fn resize(&mut self, _width: u16, _height: u16) {
        todo!()
    }

    fn handle_term_event(
        &mut self,
        event: crossterm::event::Event,
    ) -> std::result::Result<AppResult, AppError> {
        match event {
            _ => Err(AppError::NotRelevant),
        }
    }

    fn request(&mut self) -> super::Requirements {
        todo!()
    }

    fn allocate(&mut self, allocation: Rect) {
        self.area = allocation;
        info!("allocate {:?}", &self.area);
    }

    fn handle_app_request(&mut self, req: &AppRequest) -> Result<AppResult, AppError> {
        match req {
            SetValue { name, value } => {
                if name == &self.name {
                    self.txt = value.clone();
                    Ok(Redraw)
                } else {
                    Err(NotRelevant)
                }
            }
            _ => Err(NotRelevant),
        }
    }

    fn hit(&mut self, x: u16, y: u16) -> super::AppResponse {
        Err(NotRelevant)
    }

    fn allocated(&self) -> bool {
        true
    }
}
