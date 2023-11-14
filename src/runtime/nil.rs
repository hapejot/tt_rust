use std::rc::Rc;

use once_cell::sync::Lazy;

use super::{Object, ObjectPtr, Receiver};

pub static NIL: Lazy<ObjectPtr> = Lazy::new(|| Object::new());

pub struct NilReciever;

impl Receiver for NilReciever {
    fn receive_message(&self, selector: &'static str, _args: &[Rc<dyn Receiver>]) -> Rc<dyn Receiver> {
        todo!("implement {} for nil", selector)
    }

    fn as_int(&self) -> Option<isize> {
        None
    }
    fn as_str(&self) -> Option<&'static str> {
        None
    }
}
// pub static NIL_RECIEVER: Lazy<Rc<dyn Receiver>> = Lazy::new(|| Rc::new(NilReciever));
impl NilReciever {
    pub fn get() -> Rc<dyn Receiver> {
        Rc::new(NilReciever)
    }
}
