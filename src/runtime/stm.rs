use std::{rc::Rc, sync::Mutex};

use super::{int::IntReceiver, str::StringReceiver, Receiver};

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

    fn next_element(&self) -> Rc<dyn Receiver> {
        let idxobj;
        {
            let mut idx = self.idx.lock().unwrap();
            idxobj = Rc::new(IntReceiver::new(*idx));
            *idx += 1;
        }
        self.buf.receive_message("basicAt:", vec![idxobj])
    }

    fn next_put(&self, args: &Vec<Rc<dyn Receiver>>) -> Rc<dyn Receiver> {
        let idxobj;
        {
            let mut idx = self.idx.lock().unwrap();
            idxobj = Rc::new(IntReceiver::new(*idx));
            *idx += 1;
        }
        self.buf
            .receive_message("basicAt:put:", vec![idxobj, args[0].clone()])
    }
}

impl Receiver for StreamReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "next" => self.next_element(),
            "nextPut:" => self.next_put(&args),
            "nextPutAll:" => self.next_put(&args),
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
            "upTo:" => {
                let end_c = char::from_u32(args[0].as_int().unwrap() as u32).unwrap();
                let mut buf = vec![];
                loop {
                    let c = char::from_u32(self.next_element().as_int().unwrap() as u32).unwrap();
                    if c == end_c {
                        break;
                    }
                    buf.push(c);
                }
                Rc::new(StringReceiver::new(buf.iter().collect()))
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
