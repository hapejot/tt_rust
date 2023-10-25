use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    event::Event::{self},
    style::Print,
    QueueableCommand,
};
use tracing::*;

use super::{
    AppError::{self, NotRelevant},
    AppRequest, AppResult, Glyph, Rect,
};

pub struct Panel {
    area: Rect,
    elements: Vec<Box<dyn Glyph>>,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            area: Rect::new(),
            elements: vec![],
        }
    }
    pub fn add(&mut self, g: Box<dyn Glyph>) {
        self.elements.push(g);
    }
    fn write_width_markers(&self, w: &mut dyn Write) {
        for i in 1..=self.area.w {
            let label: Vec<char> = format!("{i:03}").chars().collect();
            w.queue(MoveTo(self.area.x + i - 1, self.area.y + 1))
                .unwrap();
            w.queue(Print(label[0])).unwrap();
            w.queue(MoveTo(self.area.x + i - 1, self.area.y + 2))
                .unwrap();
            w.queue(Print(label[1])).unwrap();
            w.queue(MoveTo(self.area.x + i - 1, self.area.y + 3))
                .unwrap();
            w.queue(Print(label[2])).unwrap();
        }
    }
}

impl Glyph for Panel {
    fn resize(&mut self, width: u16, height: u16) {
        self.area = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
    }
    fn write_to(&self, w: &mut dyn Write) {
        for x in self.elements.iter() {
            x.write_to(w);
        }
    }
    fn area(&self) -> super::Rect {
        self.area.clone()
    }
    fn handle_term_event(&mut self, event: Event) -> std::result::Result<AppResult, AppError> {
        match event {
            r => {
                let mut handled = Err(AppError::NotRelevant);
                for x in self.elements.iter_mut() {
                    handled = x.handle_term_event(r.clone());
                    if handled.is_ok() {
                        break;
                    }
                }
                handled
            }
        }
    }
    fn request(&mut self) -> super::Requirements {
        todo!()
    }
    fn allocate(&mut self, allocation: Rect) {
        self.area = allocation.clone();
        info!("allocate {:?}", self.area);
        let mut y = self.area.y;
        for x in self.elements.iter_mut() {
            x.allocate(Rect {
                x: allocation.x,
                y: y,
                w: allocation.w,
                h: 1,
            });
            y += 1;
        }
    }
    fn handle_app_request(&mut self, req: &AppRequest) -> Result<AppResult, AppError> {
        let mut result = Err(NotRelevant);
        for x in self.elements.iter_mut() {
            let r = x.handle_app_request(req);
            if let Ok(res) = r {
                match res {
                    AppResult::Values(vs) => {
                        if let Ok(AppResult::Values(old_vs)) = result {
                            result = Ok(AppResult::Values(
                                old_vs
                                    .into_iter()
                                    .chain(vs.into_iter())
                                    .collect(),
                            ));
                        }
                        else {
                            result = Ok(AppResult::Values(vs));
                        }
                    }
                    _ => {
                        result = Ok(res);
                        break;
                    }
                }
            }
        }
        result
    }
    fn hit(&mut self, x: u16, y: u16) -> super::AppResponse {
        let mut r = Err(NotRelevant);
        for el in self.elements.iter_mut() {
            let el_result = el.hit(x, y);
            if el_result.is_ok() {
                r = el_result.clone();
            }
        }
        r
    }

    fn allocated(&self) -> bool {
        self.area.w > 0 && self.area.h > 0
    }
}
