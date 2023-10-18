use super::{Rect, Glyph};

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
        w.write(self.txt.as_bytes()).unwrap();
    }

    fn area(&self) -> Rect {
        todo!()
    }

    fn resize(&mut self, width: u16, height: u16) {
        todo!()
    }

    fn handle_event(&mut self, event: crossterm::event::Event) -> bool {
        match event {
            _ => false,
        }
    }

    fn request(&mut self) -> super::Requirements {
        todo!()
    }

    fn allocate(&mut self, allocation: Rect) {
        todo!()
    }
}