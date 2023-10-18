use crossterm::{cursor::MoveTo, event::Event, style::Print, QueueableCommand};

use super::{Glyph, Rect};

pub struct Frame {
    area: Rect,
    content: Box<dyn Glyph>,
}

impl Frame {
    pub fn new(content: Box<dyn Glyph>) -> Self {
        Self {
            area: Rect::new(),
            content,
        }
    }
}

impl Glyph for Frame {
    fn write_to(&self, w: &mut dyn std::io::Write) {
        self.content.write_to(w);
        let r = self.area();

        w.queue(MoveTo(r.x - 1, r.y - 1)).unwrap();
        w.queue(Print("┌")).unwrap();
        for _i in 0..r.w {
            w.queue(Print("─")).unwrap();
        }
        w.queue(Print("┐")).unwrap();

        for y in r.y..r.y + r.h + 2 {
            w.queue(MoveTo(r.x - 1, y)).unwrap();
            w.queue(Print("│")).unwrap();
            w.queue(MoveTo(r.x + r.w, y)).unwrap();
            w.queue(Print("│")).unwrap();
        }

        w.queue(MoveTo(r.x - 1, r.y + r.h + 2)).unwrap();
        w.queue(Print("└")).unwrap();
        for _i in 0..r.w {
            w.queue(Print("─")).unwrap();
        }
        w.queue(Print("┘")).unwrap();
    }

    fn area(&self) -> super::Rect {
        let r = self.content.area();
        super::Rect {
            x: r.x + 1,
            y: r.y + 1,
            w: r.w - 2,
            h: r.h - 2,
        }
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.content.resize(width, height);
    }

    fn handle_event(&mut self, r: Event)->bool {
        match r {
            Event::Resize(w, h) => {
                self.allocate(super::Rect { x: 0, y: 0, w, h });
                true
            }
            x => self.content.handle_event(x),
        }
    }

    fn request(&mut self) -> super::Requirements {
        todo!()
    }

    fn allocate(&mut self, allocation: super::Rect) {
        self.area = allocation.clone();
        self.content.allocate(Rect {
            x: allocation.x + 1,
            y: allocation.y + 1,
            w: allocation.w - 2,
            h: allocation.h - 2,
        });
    }
}
