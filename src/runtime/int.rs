use core::panic;
use std::rc::Rc;

use super::{
    boo::{FalseReceiver, TrueReceiver},
    pnt::PointReceiver,
    sel::SelectorSet,
    str::StringReceiver,
    Receiver,
};

pub struct IntMetaReceiver;

pub struct IntReceiver(isize);

impl Receiver for IntMetaReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "readFrom:ifFail:" => {
                let n: isize = isize::from_str_radix(args[0].as_str().unwrap(), 10).unwrap();
                Rc::new(IntReceiver(n))
            }
            _ => todo!("int meta select {}", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}

impl IntReceiver {
    pub fn new(n: isize) -> Self {
        Self(n)
    }
}

impl Receiver for IntReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "==" => {
                if self.0 == args[0].as_int().unwrap() {
                    TrueReceiver::get()
                } else {
                    FalseReceiver::get()
                }
            }
            "+" => Rc::new(IntReceiver(self.0 + args[0].as_int().unwrap())),
            "*" => Rc::new(IntReceiver(self.0 * args[0].as_int().unwrap())),
            "<" => {
                if self.0 < args[0].as_int().unwrap() {
                    TrueReceiver::get()
                } else {
                    FalseReceiver::get()
                }
            }
            ">" => {
                if self.0 > args[0].as_int().unwrap() {
                    TrueReceiver::get()
                } else {
                    FalseReceiver::get()
                }
            }
            "asString" => Rc::new(StringReceiver::new(format!("{}", self.0))),
            "basic_write_to" => {
                let a0 = StringReceiver::new(format!("{}", self.0));
                args[0].receive_message("write", vec![Rc::new(a0)])
            }
            "@" => Rc::new(PointReceiver::new(self.0, args[0].as_int().unwrap())),
            _ => todo!("message '{}' for int", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        Some(self.0)
    }
    fn as_str(&self) -> Option<&'static str> {
        panic!("use asString instead.")
        // Some(SelectorSet::get(format!("{}", self.0).as_str()))
    }
}
