use std::rc::Rc;

use super::{
    boo::{FalseReceiver, TrueReceiver},
    sel::SelectorSet,
    str::StringReceiver,
    Receiver,
};

pub struct CharReceiver(char);

impl CharReceiver {
    pub fn new(c: char) -> Self {
        Self(c)
    }
}

impl Receiver for CharReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "==" => {
                if self.0 == char::from_u32(args[0].as_int().unwrap().try_into().unwrap()).unwrap()
                {
                    TrueReceiver::get()
                } else {
                    FalseReceiver::get()
                }
            }
            "basic_write_to" => {
                let a0 = StringReceiver::new(format!("{}", self.0));
                args[0].receive_message("write", vec![Rc::new(a0)])
            }
            _ => todo!("message '{}' for char", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        Some(self.0 as isize)
    }
    
    fn as_str(&self) -> Option<&'static str> {
        let s = String::from(self.0);
        let s1 = SelectorSet::get(s.as_str());
        Some(s1)
    }
}
