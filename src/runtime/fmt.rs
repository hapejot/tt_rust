use super::Receiver;
use crate::runtime::nil::NilReciever;
use std::{rc::Rc, sync::Mutex};

pub struct Formatter<'a, 'b> {
    f: Mutex<&'a mut std::fmt::Formatter<'b>>,
}

impl<'a, 'b> Formatter<'a, 'b> {
    pub fn new(f: &'a mut std::fmt::Formatter<'b>) -> Self {
        Self { f: Mutex::new(f) }
    }
}

impl Receiver for Formatter<'_, '_> {
    fn receive_message(&self, selector: &'static str, _args: Vec<Rc<dyn Receiver>>) -> Rc<dyn Receiver> {
        match selector {
            "write" => {
                let mut f = self.f.lock().unwrap();
                write!(f, "{}", _args[0].as_str().unwrap()).unwrap();
                NilReciever::get()
            }
            _ => todo!("message {} for Formatter", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}
