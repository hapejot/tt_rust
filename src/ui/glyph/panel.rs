use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    event::Event::{self, FocusGained, FocusLost, Key, Mouse, Paste, Resize},
    style::Print,
    QueueableCommand,
};

use super::{Glyph, Rect};

pub struct Panel {
    area: Rect,
    elements: Vec<Box<dyn Glyph>>,
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
        for x in self.elements.iter(){
            x.write_to(w);
        }
    }

    fn area(&self) -> super::Rect {
        self.area.clone()
    }

    fn handle_event(&mut self, event: Event) -> bool {
        match event {
            r => {
                let mut handled = false;
                for x in self.elements.iter_mut() {
                    handled = x.handle_event(r.clone());
                    if handled {
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
        self.area = allocation;
    }
}
