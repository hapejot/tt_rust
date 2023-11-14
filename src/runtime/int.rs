use std::rc::Rc;

use super::{pnt::PointReceiver, str::StringReceiver, Receiver};

pub struct IntReceiver(isize);

impl IntReceiver {
    pub fn new(n: isize) -> Self {
        Self(n)
    }
}

impl Receiver for IntReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: &[Rc<dyn Receiver>],
    ) -> Rc<dyn Receiver> {
        match selector {
            "+" => Rc::new(IntReceiver(self.0 + args[0].as_int().unwrap())),
            "*" => Rc::new(IntReceiver(self.0 * args[0].as_int().unwrap())),
            "basic_write_to" => {
                let a0 = StringReceiver(format!("{}", self.0));
                args[0].receive_message("write", &[Rc::new(a0)])
            }
            "@" => Rc::new(PointReceiver::new(self.0, args[0].as_int().unwrap())),
            _ => todo!("message '{}' for int", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        Some(self.0)
    }
    fn as_str(&self) -> Option<&'static str> {
        None
    }
}
