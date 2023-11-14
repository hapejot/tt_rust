use std::{fs::File, io::Read, path::Path, rc::Rc};

use tracing::info;

use crate::{parse_method, parser::AST, Context};

use super::{sel::SelectorSet, Receiver, int::IntReceiver};

pub struct StringReceiver(pub String);

impl Receiver for StringReceiver {
    fn receive_message(&self, selector: &'static str, _args: &[Rc<dyn Receiver>]) -> Rc<dyn Receiver> {
        match selector {
            "size" => Rc::new(IntReceiver::new(self.0.len()as isize)),
            "write" => Rc::new(StringReceiver(String::from(_args[0].as_str().unwrap()))),
            _ => self.execute_stored_method(selector)
        }
        
        // todo!("implement {} for str", selector)
    }

    fn as_int(&self) -> Option<isize> {
        match str::parse(self.0.as_str()) {
            Ok(i) => Some(i),
            Err(_) => None,
        }
    }
    fn as_str(&self) -> Option<&'static str> {
        Some(SelectorSet::get(self.0.as_str()))
    }
}

impl StringReceiver {
    fn execute_stored_method(&self, selector: &str) -> Rc<dyn Receiver> {
        let p = format!("defs/string/{}", selector).replace(r":", "_");
        let p = Path::new(&p);
        if !p.exists() {
            panic!("unresolved method {}", selector);
        }
        
        let mut f = File::open(p).unwrap();
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        let m = parse_method(buf).unwrap()[0]
            .clone()
            .as_abstract_syntax_tree();
        match m {
            AST::Method {
                name,
                params,
                temps,
                body,
            } => {
                let myself: Rc<dyn Receiver> = Rc::new(StringReceiver(self.0.clone()));
                let mut ctx = Context::new(myself);
                info!("name: {}", name);
                info!("params: {:?}", params);
                info!("temps: {:?}", temps);
                ctx.eval_to_reciever(&body)
            }
            _ => todo!("I only know how to deal with a method."),
        }
    }
}
