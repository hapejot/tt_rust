use std::{fs::File, io::Read, path::Path, rc::Rc, sync::Mutex};

use tracing::info;

use crate::{parse_method, parser::AST, runtime::stm::StreamReceiver, Context};

use super::{int::IntReceiver, sel::SelectorSet, Receiver};

pub struct StringMetaReceiver {}

impl Receiver for StringMetaReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: &[Rc<dyn Receiver>],
    ) -> Rc<dyn Receiver> {
        match selector {
            "new:streamContents:" => {
                let result = Rc::new(StringReceiver::new(String::new()));
                let stream = Rc::new(StreamReceiver::new(result.clone()));
                args[1].receive_message("value:", &[stream]);
                result
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

pub struct StringReceiver {
    val: Mutex<String>,
}

impl Receiver for StringReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        _args: &[Rc<dyn Receiver>],
    ) -> Rc<dyn Receiver> {
        match selector {
            "species" => Rc::new(StringMetaReceiver {}),
            "size" => Rc::new(IntReceiver::new(self.val.lock().unwrap().len() as isize)),
            "readStream" => {
                let s = self.val.lock().unwrap();
                let stream = Rc::new(StreamReceiver::new(Rc::new(StringReceiver::new(s.clone()))));
                stream
            }
            "basicAt:" => {
                let s = self.val.lock().unwrap();
                let idx = _args[0].as_int().unwrap();
                let c = s.chars().nth(idx as usize).unwrap();
                Rc::new(IntReceiver::new(c as isize))
            }
            "basicAt:put:" => {
                let result = _args[1].clone();
                {
                    let mut s = self.val.lock().unwrap();
                    let idx = _args[0].as_int().unwrap();
                    s.insert(
                        idx as usize,
                        char::from_u32(result.as_int().unwrap() as u32).unwrap(),
                    );
                }
                result
            }
            "write" => {
                let mut s = self.val.lock().unwrap();
                s.push_str(_args[0].as_str().unwrap());
                _args[0].clone()
            }
            "basic_write_to" => {
                let s = {
                    let content = self.val.lock().unwrap();
                    content.clone()
                };
                _args[0].receive_message("write", &[Rc::new(StringReceiver::new(s))])
            }
            _ => self.execute_stored_method(selector),
        }

        // todo!("implement {} for str", selector)
    }

    fn as_int(&self) -> Option<isize> {
        match str::parse(self.val.lock().unwrap().as_str()) {
            Ok(i) => Some(i),
            Err(_) => None,
        }
    }
    fn as_str(&self) -> Option<&'static str> {
        Some(SelectorSet::get(self.val.lock().unwrap().as_str()))
    }
}

impl StringReceiver {
    pub fn new(val: String) -> Self {
        Self {
            val: Mutex::new(val),
        }
    }

    fn execute_stored_method(&self, selector: &str) -> Rc<dyn Receiver> {
        let p = format!("defs/string/{}", selector).replace(r":", "_");
        let p = Path::new(&p);
        if !p.exists() {
            panic!("unresolved method {}", selector);
        }

        let mut f = File::open(p).unwrap();
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();

        let t = parse_method(buf).unwrap()[0].clone();
        // trace!("tree: {}", t);
        let m = t.as_abstract_syntax_tree();
        match m {
            AST::Method {
                name,
                params,
                temps,
                body,
            } => {
                let myself: Rc<dyn Receiver> =
                    Rc::new(StringReceiver::new(self.val.lock().unwrap().clone()));
                let mut ctx = Context::new(myself);
                info!("name: {}", name);
                info!("params: {:?}", params);
                info!("temps: {:?}", temps);
                // trace!("AST: {:#?}", &body);
                ctx.eval_to_reciever(&body)
            }
            _ => todo!("I only know how to deal with a method."),
        }
    }
}
