pub mod sel;

use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub enum Address {
    Instance(i32),
    Temporary(i32),
    Literal(Literal),
    Receiver,
    Super,
}

#[derive(Debug)]
pub enum Literal {
    String(String),
    Int64(i64),
    Int8(i8),
    U8(u8),
    U64(u64),
}

#[derive(Debug)]
pub enum Instruction {
    Return(Address),
}

#[allow(dead_code)]

pub type ObjectVec<'a> = &'a [ObjectPtr];
pub type Instructions = Vec<Instruction>;

#[derive(Clone)]
pub struct ObjectPtr {
    ptr: Arc<Object>,
}
impl ObjectPtr {
    pub fn send(&self, name: &'static str, as_slice: &[ObjectPtr]) -> ObjectPtr {
        let o = self.ptr.data.lock().unwrap();

        let handler = o.handler;
        handler(name, self.clone(), as_slice)
    }
}

impl PartialEq for ObjectPtr {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(Arc::as_ptr(&self.ptr), Arc::as_ptr(&other.ptr))
    }
}

impl std::fmt::Debug for ObjectPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = Box::new(format!("{:x}", Arc::as_ptr(&self.ptr) as u64));
        f.debug_struct("ObjectPtr").field("ptr", &s).finish()
    }
}

impl Display for ObjectPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Object[]")
    }
}

pub struct Frame<'a> {
    pub machine: &'a Machine,
    pub receiver: ObjectPtr,
    pub next_instruction: usize,
    pub locals: Vec<ObjectPtr>,
    pub proc: Box<Procedure>,
    pub result: ObjectPtr,
}
impl<'a> Frame<'a> {
    pub fn new(m: &'a Machine, receiver: ObjectPtr, proc: Box<Procedure>) -> Frame<'a> {
        let locals = (1..proc.slot_count)
            .map(|_| m.nil.clone())
            .collect::<Vec<_>>();
        Frame {
            machine: m,
            receiver: receiver,
            next_instruction: 0,
            locals: locals,
            proc: proc,
            result: m.nil.clone(),
        }
    }
}
#[allow(dead_code)]
pub struct Procedure {
    pub slot_count: i32,
    pub instructions: Instructions,
}

pub type Handler = fn(&'static str, ObjectPtr, &[ObjectPtr]) -> ObjectPtr;

#[allow(dead_code)]
pub struct Object {
    data: Mutex<ObjectData>,
}

#[allow(dead_code)]
struct ObjectData {
    handler: Handler,
    vars: Vec<ObjectPtr>,
    literal: Option<Literal>,
}

pub trait ObjectInternals {
    fn as_str(&self) -> Option<String>;
}

impl Object {
    pub fn new() -> ObjectPtr {
        ObjectPtr {
            ptr: Arc::new(Object {
                data: Mutex::new(ObjectData {
                    handler: nil_handler,
                    vars: vec![],
                    literal: None,
                }),
            }),
        }
    }

    pub fn new_with_handler(handler: Handler) -> ObjectPtr {
        ObjectPtr {
            ptr: Arc::new(Object {
                data: Mutex::new(ObjectData {
                    handler,
                    vars: [].into(),
                    literal: None,
                }),
            }),
        }
    }

    pub fn new_string(s: &str) -> ObjectPtr {
        ObjectPtr {
            ptr: Arc::new(Object {
                data: Mutex::new(ObjectData {
                    handler: nil_handler,
                    vars: [].into(),
                    literal: Some(Literal::String(s.into())),
                }),
            }),
        }
    }
}

impl ObjectInternals for Object {
    fn as_str(&self) -> Option<String> {
        let x = self.data.lock().unwrap();
        if let Some(Literal::String(s)) = &x.literal {
            Some(s.into())
        } else {
            None
        }
    }
}

impl ObjectInternals for ObjectPtr {
    fn as_str(&self) -> Option<String> {
        self.ptr.as_str()
    }
}

pub struct Machine {
    pub nil: ObjectPtr,
}

impl Machine {
    pub fn new() -> Machine {
        let nil = Object::new();
        Machine { nil }
    }
}

fn nil_handler(_sel: &str, _s: ObjectPtr, _args: &[ObjectPtr]) -> ObjectPtr {
    _s
}

#[allow(dead_code)]
fn str_handler(_sel: &str, _s: ObjectPtr, _args: ObjectVec) -> ObjectPtr {
    _s
}

#[allow(dead_code)]
fn string_handler(_sel: &str, _s: ObjectPtr, _args: Vec<ObjectPtr>) -> ObjectPtr {
    Object::new_string("...")
}

pub fn eval(_frame: &mut Frame) -> Result<(), String> {
    let n = _frame.proc.instructions.len();
    while _frame.next_instruction < n {
        match &_frame.proc.instructions[_frame.next_instruction] {
            Instruction::Return(Address::Receiver) => {
                _frame.result = _frame.receiver.clone();
                jump_to_end(_frame);
            }
            Instruction::Return(Address::Literal(Literal::String(s))) => {
                _frame.result = Object::new_string(s);
                jump_to_end(_frame);
            }
            Instruction::Return(x) => println!("return {:?}", x),
        }
        _frame.next_instruction += 1;
    }
    Ok(())
}

fn jump_to_end(_frame: &mut Frame) {
    let n = _frame.proc.instructions.len();
    _frame.next_instruction = n;
}

#[cfg(test)]
mod test {

    

    use crate::runtime::*;

    #[test]
    fn eval_nil() {
        let m = Machine::new();
        let proc = Procedure {
            slot_count: 0,
            instructions: vec![Instruction::Return(Address::Receiver)],
        };
        let o = Object::new();
        let mut frame = Frame::new(&m, o.clone(), Box::new(proc));
        assert_eq!(eval(&mut frame), Ok(()));
        assert_eq!(frame.result, o);
    }

    #[test]
    fn eval_string_literal() {
        let m = Machine::new();
        let proc = Procedure {
            slot_count: 0,
            instructions: vec![Instruction::Return(Address::Literal(Literal::String(
                "Test".into(),
            )))],
        };
        let o = Object::new_string("Test");
        let mut frame = Frame::new(&m, o.clone(), Box::new(proc));
        assert_eq!(eval(&mut frame), Ok(()));
        let result = frame.result.as_str().unwrap();
        assert_eq!(result, o.as_str().unwrap());
    }
}
