use std::rc::Rc;

// use once_cell::sync::Lazy;

use super::{str::StringReceiver, Receiver};

// pub static TRUE: Lazy<ObjectPtr> = Lazy::new(|| Object::new());

pub struct TrueReceiver;

impl Receiver for TrueReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "ifTrue:" => args[0].receive_message("value", vec![]),
            "ifFalse:" => TrueReceiver::get(),
            "ifTrue:ifFalse:" => args[0].receive_message("value", vec![]),
            "basic_write_to" => {
                let a0 = StringReceiver::new(format!("True"));
                args[0].receive_message("write", vec![Rc::new(a0)])
            }
            _ => todo!("implement {} for True", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        Some(1)
    }
    fn as_str(&self) -> Option<&'static str> {
        Some("True")
    }
}
// pub static NIL_RECIEVER: Lazy<Rc<dyn Receiver>> = Lazy::new(|| Rc::new(NilReciever));
impl TrueReceiver {
    pub fn get() -> Rc<dyn Receiver> {
        Rc::new(TrueReceiver)
    }
}

pub struct FalseReceiver;

impl Receiver for FalseReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "basic_write_to" => {
                let a0 = StringReceiver::new(format!("False"));
                args[0].receive_message("write", vec![Rc::new(a0)])
            }
            "ifTrue:" => FalseReceiver::get(),
            "ifFalse:" => args[0].receive_message("value", vec![]),
            "ifTrue:ifFalse:" => args[1].receive_message("value", vec![]),
            _ => todo!("implement {} for False", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        Some(0)
    }
    fn as_str(&self) -> Option<&'static str> {
        Some("False")
    }
}
// pub static NIL_RECIEVER: Lazy<Rc<dyn Receiver>> = Lazy::new(|| Rc::new(NilReciever));
impl FalseReceiver {
    pub fn get() -> Rc<dyn Receiver> {
        Rc::new(FalseReceiver)
    }
}
