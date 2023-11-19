use super::{int::IntReceiver, str::StringReceiver, Receiver};
use std::rc::Rc;

pub struct PointMetaReceiver;

impl Receiver for PointMetaReceiver {
    fn receive_message(&self, selector: &'static str, args: Vec<Rc<dyn Receiver>>) -> Rc<dyn Receiver> {
        match selector {
            "x:y:" => {
                let x = args[0].as_int().unwrap();
                let y = args[1].as_int().unwrap();

                Rc::new(PointReceiver::new(x, y))
            }
            _ => todo!("{}", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}

pub struct PointReceiver(isize, isize);

impl PointReceiver {
    pub fn new(x: isize, y: isize) -> Self {
        Self(x, y)
    }
}

impl Receiver for PointReceiver {
    fn receive_message(&self, selector: &'static str, args: Vec<Rc<dyn Receiver>>) -> Rc<dyn Receiver> {
        match selector {
            "x" => Rc::new(IntReceiver::new(self.0)),
            "y" => Rc::new(IntReceiver::new(self.1)),
            "+" => {
                let arg = args[0].clone();
                let x = arg.receive_message("x", vec![]).as_int().unwrap();
                let y = arg.receive_message("y", vec![]).as_int().unwrap();
                Rc::new(PointReceiver::new(self.0 + x, self.1 + y))
            }
            "basic_write_to" => {
                let a0 = StringReceiver::new(format!("{}@{}", self.0, self.1));
                args[0].receive_message("write", vec![Rc::new(a0)])
            }
            _ => todo!("{}", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}
