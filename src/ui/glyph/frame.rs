

use crossterm::{cursor::MoveTo, style::Print, QueueableCommand};

use super::Glyph;

pub struct Frame {
    content: Box<dyn Glyph>,
}

impl Frame {
    pub fn new(content: Box<dyn Glyph>) -> Self {
        Self { content }
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

    fn handle_event(&mut self, _r: crossterm::event::Event) {
        todo!()
    }
}
