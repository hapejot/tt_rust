use std::{
    borrow::Borrow,
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use odbc_api::parameter::Text;

trait W {
    fn draw(&self);
}

struct L {
    label: TextContent,
    lang: Option<String>,
}

impl W for L {
    fn draw(&self) {
        println!("Label: {:?}/{:?}", self.label, self.lang);
    }
}

impl L {
    pub fn new<T: Into<String>>(s: T) -> L {
        L {
            label: TextContent::new(s),
            lang: None,
        }
    }

    pub fn empty() -> L {
        L {
            label: TextContent::empty(),
            lang: None,
        }
    }

    pub fn label<T: Into<String>>(&self, s: T) -> &Self {
        self
    }

    pub(crate) fn content(&self) -> TextContent {
        self.label.clone()
    }
}

#[derive(Clone, Debug)]
struct TextContent {
    s: Arc<Mutex<String>>,
}

impl TextContent {
    pub fn empty() -> TextContent {
        TextContent {
            s: Arc::new(Mutex::new("".into())),
        }
    }
    pub fn new<T:Into<String>>(s:T) -> TextContent {
        TextContent {
            s: Arc::new(Mutex::new(s.into())),
        }
    }
    pub fn replace<T: Into<String>>(&self, s: T) -> &Self {
        let mut x = self.s.lock().unwrap();
        *x = s.into();
        self
    }
}

struct Child {
    w: Box<dyn W>,
}

struct F {
    cs: Vec<Child>,
}
impl F {
    pub(crate) fn new() -> F {
        F { cs: vec![] }
    }

    pub(crate) fn add(&mut self, l0: L) {
        self.cs.push(Child { w: Box::new(l0) });
    }
}

impl W for F {
    fn draw(&self) {
        for x in self.cs.iter() {
            x.w.draw()
        }
    }
}

fn main() {
    println!("bullshit!");

    let l0 = L::new("Name");
    let l1 = L::new("Address");
    let c1 = l1.content();
    let mut f = F::new();
    f.add(l0);
    f.add(l1);
    f.draw();
    c1.replace("Addr");
    println!("new struct:");
    f.draw();
}
