use std::{fs::File, io::Read, path::Path, rc::Rc, sync::Mutex, ops::Deref};

use tracing::info;

use crate::{parse_method, parser::AST, runtime::stm::StreamReceiver, Context};

use super::{int::IntReceiver, sel::SelectorSet, Receiver};

pub struct ArrayReceiver(pub Vec<Rc<dyn Receiver>>);

impl Deref for ArrayReceiver {
    type Target = Vec<Rc<dyn Receiver>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


impl Receiver for ArrayReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "at:" => {
                let idx = args[0].as_int().unwrap();
                self[idx as usize].clone()
            }
            _ => todo!("array selector {}", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}