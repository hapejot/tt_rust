use super::AppError::*;
use super::AppRequest::*;
use super::AppResult::*;
use super::*;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::{cursor::MoveTo, style::Print, QueueableCommand};
use tracing::*;

pub struct Input {
    active: bool,
    pos: u16,
    area: Rect,
    name: String,
    txt: Vec<char>,
}

impl Input {
    pub fn new<T:ToString, U:ToString>(name: T, txt: U) -> Self {
        Self {
            active: false,
            pos: 0,
            area: Rect::new(),
            txt: txt.to_string().chars().collect(),
            name: name.to_string(),
        }
    }
}

impl Glyph for Input {
    fn write_to(&self, w: &mut dyn std::io::Write) {
        w.queue(MoveTo(self.area.x, self.area.y)).unwrap();
        let mut idx = 0;
        for c in self.txt.iter() {
            idx += 1;
            w.queue(Print(c)).unwrap();
        }
        for _ in idx..self.area.w {
            w.queue(Print('_')).unwrap();
        }
    }

    fn area(&self) -> Rect {
        todo!()
    }

    fn resize(&mut self, _width: u16, _height: u16) {
        todo!()
    }

    fn handle_term_event(&mut self, event: Event) -> std::result::Result<AppResult, AppError> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                ..
            }) => {
                if self.active {
                    if self.txt.len() >= self.pos as usize {
                        self.pos -= 1;
                        self.txt.remove(self.pos as usize);
                        Ok(NewCursorPosition(self.area.x + self.pos, self.area.y))
                    } else {
                        Ok(NewCursorPosition(self.area.x + self.pos - 1, self.area.y))
                    }
                } else {
                    Err(NotRelevant)
                }
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(ch),
                // modifiers,
                kind: KeyEventKind::Press,
                ..
            }) => {
                if self.active {
                    if (self.txt.len() as u16) <= self.pos {
                        self.txt.push(ch);
                        self.pos = self.txt.len() as u16;
                    } else {
                        self.txt[self.pos as usize] = ch;
                        self.pos += 1;
                    }
                    Ok(NewCursorPosition(self.area.x + self.pos, self.area.y))
                } else {
                    Err(NotRelevant)
                }
            }
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
                    self.txt = value.chars().collect();
                    Ok(Redraw)
                } else {
                    Err(NotRelevant)
                }
            }
            GetValue(name) => {
                if name == &self.name {
                    Ok(AppResult::StringValue(self.txt.iter().collect()))
                } else {
                    Err(NotRelevant)
                }
            }
            NextInput(_, y) => {
                if self.area.y > *y {
                    Ok(NewCursorPosition(self.area.x, self.area.y))
                } else {
                    Err(NotRelevant)
                }
            }
            CollectAllValues => Ok(Values(vec![(self.name.clone(), self.txt.iter().collect())])),
            _ => Err(NotRelevant),
        }
    }

    fn hit(&mut self, x: u16, y: u16) -> super::AppResponse {
        if self.area.contains(x, y) {
            self.active = true;
            self.pos = x - self.area.x;
            Ok(AppResult::InputEnabled)
        } else {
            self.active = false;
            Err(NotRelevant)
        }
    }

    fn allocated(&self) -> bool {
        self.area.w > 0 && self.area.h > 0
    }
}
