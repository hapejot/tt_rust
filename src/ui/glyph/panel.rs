use std::io::Write;

use crossterm::{cursor::MoveTo, style::Print, QueueableCommand};

use super::{Glyph, Rect};

pub struct Panel {
    area: Rect,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            area: Rect {
                x: 0,
                y: 0,
                w: 80,
                h: 25,
            },
        }
    }
}

impl Glyph for Panel {
    fn write_to(&self, w: &mut dyn Write) {
        for i in 1..=self.area.w {
            let label: Vec<char> = format!("{i:02}").chars().collect();
            w.queue(MoveTo(self.area.x + i - 1, self.area.y + 1))
                .unwrap();
            w.queue(Print(label[0])).unwrap();
            w.queue(MoveTo(self.area.x + i - 1, self.area.y + 2))
                .unwrap();
            w.queue(Print(label[1])).unwrap();
        }
    }

    fn area(&self) -> super::Rect {
        self.area.clone()
    }
}
