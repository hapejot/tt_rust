use std::{
    io::Write,
    sync::{Arc, Mutex}, rc::Rc,
};

use crossterm::{
    cursor::{position, MoveTo},
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode},
    execute, queue,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen, SetTitle,
    },
    QueueableCommand, Result,
};

#[derive(Clone)]
pub struct TextContent {
    pub content: Arc<Mutex<String>>,
}

impl TextContent {
    pub fn new<T: Into<String>>(content: T) -> Self {
        TextContent {
            content: Arc::new(Mutex::new(content.into())),
        }
    }

    fn empty() -> TextContent {
        Self::new("")
    }
}

pub trait Widget {
    fn draw(&self, f: &mut Box<dyn Write>);
    fn handle_event(&self, e: Event) -> Option<Event>;
}

pub struct Location {
    pub x: u16,
    pub y: u16,
}

pub struct Label {
    pub txt: TextContent,
    pub location: Location,
}
impl Label {
    pub fn new<T: Into<String>>(arg: T) -> Label {
        Label {
            txt: TextContent::new(arg),
            location: Location { x: 0, y: 0 },
        }
    }
    pub fn content(&self) -> TextContent {
        self.txt.clone()
    }
}

impl Widget for Label {
    fn draw(&self, f: &mut Box<dyn Write>) {
        let _ = queue!(f, MoveTo(self.location.x, self.location.y));
        let c0 = self.txt.content.clone();
        let c = c0.lock().unwrap();
        let _ = f.write((*c).as_bytes());
    }

    fn handle_event(&self, e: Event) -> Option<Event> {
        Some(e)
    }
}
pub struct TextInput {
    pub txt: TextContent,
    pub location: Location,
    pub width: u16,
}
impl TextInput {
    pub fn new(arg: u16) -> TextInput {
        TextInput {
            txt: TextContent::empty(),
            location: Location { x: 0, y: 0 },
            width: arg,
        }
    }
}

impl Widget for TextInput {
    fn draw(&self, f: &mut Box<dyn Write>) {
        let _ = queue!(f, MoveTo(self.location.x, self.location.y));
        let c0 = self.txt.content.clone();
        if let Ok(c) = c0.try_lock() {
            let _ = f.write(c.as_bytes());
        }
        let _ = queue!(f, MoveTo(self.location.x + self.width, self.location.y));
        let _ = f.flush();
        let _ = f.write("]".as_bytes());
    }

    fn handle_event(&self, e: Event) -> Option<Event> {
        Some(e)
    }
}

pub struct Form {
    v: Vec<WidgetRef>,
}

impl Form {
    pub fn add(&mut self, w: WidgetRef) {
        self.v.push(w);
    }
    pub fn new() -> Form {
        Form { v: vec![] }
    }
}

impl Widget for Form {
    fn draw(&self, f: &mut Box<dyn Write>) {
        for x in self.v.iter() {
            x.draw(f);
        }
    }
    fn handle_event(&self, e: Event) -> Option<Event> {
        Some(e)
    }
}

#[derive(Clone)]
pub struct WidgetRef {
    w: Rc<dyn Widget>,
}

impl Widget for WidgetRef {
    fn draw(&self, f: &mut Box<dyn Write>) {
        self.w.draw(f)
    }

    fn handle_event(&self, e: Event) -> Option<Event> {
        self.w.handle_event(e)
    }
}

