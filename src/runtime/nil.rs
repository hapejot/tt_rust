use std::rc::Rc;

use once_cell::sync::Lazy;

use super::{Object, ObjectPtr, Receiver, str::StringReceiver};

pub static NIL: Lazy<ObjectPtr> = Lazy::new(|| Object::new());

pub struct NilReciever;

impl Receiver for NilReciever {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "basic_write_to" => {
                let a0 = StringReceiver::new(format!("Nil"));
                args[0].receive_message("write", vec![Rc::new(a0)])
            }
            _ => todo!("implement {} for nil", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }
    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}
// pub static NIL_RECIEVER: Lazy<Rc<dyn Receiver>> = Lazy::new(|| Rc::new(NilReciever));
impl NilReciever {
    pub fn get() -> Rc<dyn Receiver> {
        Rc::new(NilReciever)
    }
}
