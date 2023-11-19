use std::{rc::Rc, sync::Mutex};

use crate::{parser::AST, Context};

use super::{nil::NilReciever, str::StringReceiver, Receiver};

pub struct BlockReceiver {
    params: Vec<&'static str>,
    temps: Vec<&'static str>,
    body: Box<AST>,
    ctx: Mutex<Context>,
}

impl BlockReceiver {
    pub fn new(
        myself: Rc<dyn Receiver>,
        params: &[&'static str],
        temps: &[&'static str],
        body: Box<AST>,
    ) -> Self {
        let ctx = Context::new(myself);
        Self {
            params: params.into(),
            temps: temps.into(),
            ctx: Mutex::new(ctx),
            body,
        }
    }

    pub(crate) fn define(&self, name: &'static str, rec: Rc<dyn Receiver>) {
        let ctx = self.ctx.lock().unwrap();
        ctx.set_receiver(name, rec)
    }
}

impl Receiver for BlockReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> std::rc::Rc<dyn Receiver> {
        let _ = (&self.params, &self.temps, &self.body);
        match selector {
            "value:value:" => {
                let mut ctx = self.ctx.lock().unwrap();
                assert_eq!(self.params.len(), 2);
                ctx.set_receiver(self.params[0], args[0].clone());
                ctx.set_receiver(self.params[1], args[1].clone());
                let r = ctx.eval_to_reciever(&self.body);
                r
            }
            "value:" => {
                let mut ctx = self.ctx.lock().unwrap();
                assert_eq!(self.params.len(), 1);
                ctx.set_receiver(self.params[0], args[0].clone());
                let r = ctx.eval_to_reciever(&self.body);
                r
            }
            "value" => {
                let mut ctx = self.ctx.lock().unwrap();
                let r = ctx.eval_to_reciever(&self.body);
                r
            }
            "whileFalse:" => {
                let mut ctx = self.ctx.lock().unwrap();
                loop {
                    let r = ctx.eval_to_reciever(&self.body);
                    if r.as_int().unwrap() > 0 {
                        break;
                    }
                    let _x = args[0].receive_message("value", vec![]);
                }
                NilReciever::get()
            }
            "basic_write_to" => {
                let a0 = StringReceiver::new(format!("[{:?}]", self.params));
                args[0].receive_message("write", vec![Rc::new(a0)])
            }
            _ => todo!("selector {}", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}
