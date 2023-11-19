use std::{rc::Rc, sync::Mutex};

use super::{int::IntReceiver, Receiver};

pub struct StreamReceiver {
    buf: Rc<dyn Receiver>,
    idx: Mutex<isize>,
}

impl StreamReceiver {
    pub fn new(buf: Rc<dyn Receiver>) -> Self {
        Self {
            buf,
            idx: Mutex::new(0),
        }
    }
}

impl Receiver for StreamReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "next" => {
                let idxobj;
                {
                    let mut idx = self.idx.lock().unwrap();
                    idxobj = Rc::new(IntReceiver::new(*idx));
                    *idx += 1;
                }
                self.buf
                    .receive_message("basicAt:", vec![idxobj])
            }
            "nextPut:" => {
                let idxobj;
                {
                    let mut idx = self.idx.lock().unwrap();
                    idxobj = Rc::new(IntReceiver::new(*idx));
                    *idx += 1;
                }
                self.buf
                    .receive_message("basicAt:put:", vec![idxobj, args[0].clone()])
            }
            "atEnd" => {
                let n = self.buf.receive_message("size", vec![]);
                {
                    let idx = self.idx.lock().unwrap();
                    if *idx < n.as_int().unwrap() {
                        Rc::new(IntReceiver::new(0))
                    } else {
                        Rc::new(IntReceiver::new(1))
                    }
                }
            }
            _ => todo!("method {}", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}
