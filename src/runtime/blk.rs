use std::rc::Rc;

use crate::{parser::AST, Context};

use super::{Receiver, nil::NilReciever};

pub struct BlockReceiver {
    params: Vec<&'static str>,
    temps: Vec<&'static str>,
    body: Box<AST>,
}

impl BlockReceiver {
    pub fn new(params: &[&'static str], temps: &[&'static str], body: Box<AST>) -> Self {
        Self {
            params: params.into(),
            temps: temps.into(),
            body,
        }
    }
}


impl Receiver for BlockReceiver {
    fn receive_message(&self, selector: &'static str, args: &[Rc<dyn Receiver>]) -> std::rc::Rc<dyn Receiver> {
        let _ = (args, &self.params, &self.temps, &self.body);
        match selector {
            "value:value:" => {
                let mut ctx = Context::new(NilReciever::get());
                assert_eq!(self.params.len(), 2);
                ctx.set_receiver(self.params[0], args[0].clone());
                ctx.set_receiver(self.params[1], args[1].clone());
                let r = ctx.eval_to_reciever(&self.body);
                r
            }
            "value:" => {
                let mut ctx = Context::new(NilReciever::get());
                assert_eq!(self.params.len(), 1);
                ctx.set_receiver(self.params[0], args[0].clone());
                let r = ctx.eval_to_reciever(&self.body);
                r
            }
            "value" => {
                let mut ctx = Context::new(NilReciever::get());
                let r = ctx.eval_to_reciever(&self.body);
                r
            }
            _ => todo!("selector {}", selector)
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}