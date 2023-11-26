use std::{collections::BTreeMap, fmt::Display, rc::Rc, sync::Arc};

use code::CodeAddress;
use parser::AST;
use runtime::{
    arr::ArrayReceiver,
    blk::BlockReceiver,
    int::{IntMetaReceiver, IntReceiver},
    nil::NilReciever,
    pnt::PointMetaReceiver,
    sel::SelectorSet,
    str::StringReceiver,
    Object, ObjectPtr, Receiver, chr::CharReceiver,
};

use santiago::{
    lexer::{lex, Lexeme, LexerError, Position},
    parser::{parse, ParseError, Tree},
};
use std::{path::Path, sync::Mutex};
use tracing::info;

use once_cell::sync::Lazy;

pub mod code;
pub mod controls;
pub mod data;
pub mod dbx;
pub mod error;
pub mod parser;
pub mod runtime;
pub mod tsort;
pub mod ui;

pub static TRACING: Lazy<bool> = Lazy::new(init_tracing);

pub fn init_tracing() -> bool {
    use tracing_subscriber::filter::LevelFilter;
    // let n = format!("test-{}.log", chrono::Utc::now());
    let path = Path::new("tracing.log");
    let log_file = std::fs::File::create(path).unwrap();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    true
}

struct AppError {
    msg: Box<dyn std::fmt::Display>,
}

#[derive(Clone)]
pub struct MethodContext(Arc<FrameData>);

pub struct BlockContext {
    // start: CodeAddress,
    parent: ContextRef,
    // params: Mutex<Vec<Rc<dyn Receiver>>>,
}

// pub struct Frame(Mutex<FrameData>);

pub struct FrameData {
    instruction_pointer: Mutex<CodeAddress>,
    // stack: Vec<CodeAddress>,
    // done: bool,
    // method: Rc<CompiledMethod>,
    values: Mutex<BTreeMap<CodeAddress, Rc<dyn Receiver>>>,
}

struct Context {
    receiver_names: Mutex<BTreeMap<&'static str, Rc<dyn Receiver>>>,
    myself: Rc<dyn Receiver>,
}

pub type ContextRef = Rc<dyn ContextTrait>;

// pub struct ContextRef {
//     ctx: Rc<dyn ContextTrait>,
// }

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.msg.fmt(f)
    }
}

impl std::fmt::Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AppError<")?;
        self.msg.fmt(f)?;
        write!(f, ">")
    }
}
impl std::error::Error for AppError {}

pub fn evaluate_script(
    input_string: String,
) -> Result<Rc<dyn Receiver>, Box<dyn std::error::Error>> {
    let parse_trees = parse_script(input_string)?;
    let mut ctx = Context::new(NilReciever::get());
    let mut o = NilReciever::get();
    for t in parse_trees {
        let ast = t.as_abstract_syntax_tree();
        println!("-> {}", t.to_string());
        println!("-> {:#?}", &ast);
        o = ctx.eval_to_reciever(&ast);
        println!("eval -> {}", o);
    }
    Ok(o)
}

pub trait ContextTrait {
    fn ip(&self) -> CodeAddress;
    fn next_ip(&self);
    fn call(&self, addr: CodeAddress);
    fn get_value(&self, addr: &CodeAddress) -> Rc<dyn Receiver>;
    fn get_values(&self, addrs: &[CodeAddress]) -> Vec<Rc<dyn Receiver>>;
    fn set_value(&self, addr: &CodeAddress, value: Rc<dyn Receiver>);
}

impl ContextTrait for BlockContext {
    fn ip(&self) -> CodeAddress {
        self.parent.ip()
    }

    fn next_ip(&self) {
        self.parent.next_ip()
    }

    fn get_value(&self, addr: &CodeAddress) -> Rc<dyn Receiver> {
        self.parent.get_value(addr)
    }

    fn get_values(&self, addrs: &[CodeAddress]) -> Vec<Rc<dyn Receiver>> {
        self.parent.get_values(addrs)
    }

    fn set_value(&self, addr: &CodeAddress, value: Rc<dyn Receiver>) {
        self.parent.set_value(addr, value);
    }

    fn call(&self, addr: CodeAddress) {
        self.parent.call(addr)
    }
}

// impl Frame {
//     pub fn lock(&self) -> MutexGuard<'_, FrameData> {
//         match self.0.try_lock() {
//             Ok(l) => l,
//             Err(e) => {
//                 println!("try lock: {}", e);
//                 panic!();
//             },
//         }
//     }
// }

impl ContextTrait for MethodContext {
    fn ip(&self) -> CodeAddress {
        self.0.instruction_pointer.lock().unwrap().clone()
    }

    fn next_ip(&self) {
        let ip = &mut self.0.instruction_pointer.lock().unwrap();
        ip.1 += 1;
    }

    fn get_value(&self, addr: &CodeAddress) -> Rc<dyn Receiver> {
        let vs = self.0.values.lock().unwrap();
        match vs.get(addr) {
            Some(val) => val.clone(),
            None => panic!("undefined value {}", addr),
        }
    }

    fn get_values(&self, addrs: &[CodeAddress]) -> Vec<Rc<dyn Receiver>> {
        let mut result = vec![];
        let vs = self.0.values.lock().unwrap();
        for addr in addrs {
            result.push(vs.get(addr).unwrap().clone());
        }
        result
    }

    fn set_value(&self, addr: &CodeAddress, value: Rc<dyn Receiver>) {
        let vs = &mut self.0.values.lock().unwrap();
        vs.insert(addr.clone(), value);
    }

    fn call(&self, addr: CodeAddress) {
        let ip = &mut self.0.instruction_pointer.lock().unwrap();
        **ip = addr;
    }
}

impl MethodContext {
    pub fn new() -> ContextRef {
        let c1 = MethodContext(Arc::new(FrameData::new()));
        let ctx: Rc<dyn ContextTrait> = Rc::new(c1);
        ctx
    }

    fn done(&self) -> bool {
        (*self.0).done()
    }

    fn set_ip(&self, start: CodeAddress) -> CodeAddress {
        (*self.0).set_ip(start)
    }

    fn result(&self, idx: usize) -> Rc<dyn Receiver> {
        self.0.result(idx)
    }
}

impl FrameData {
    pub fn new() -> Self {
        Self {
            instruction_pointer: Mutex::new(CodeAddress(0, 0)),
            values: Mutex::new(BTreeMap::new()),
        }
    }

    // pub fn run(&mut self) -> Rc<dyn Receiver> {
    //     while !self.done {
    //         self.process_step();
    //     }
    //     match self.method.blocks[0].result {
    //         Some(result_addr) => match self.values.get(&result_addr) {
    //             Some(v) => v.clone(),
    //             None => panic!("value {} undefined.", result_addr),
    //         },
    //         None => todo!(),
    //     }
    // }

    // fn process_step(&self, method_context: ContextRef) {
    //     let op;
    //     let block;
    //     let step;
    //     {
    //         let CodeAddress(ip_block, ip_step) = *self.ip.try_lock().unwrap();
    //         block = ip_block;
    //         step = ip_step;
    //         op = &self.method.blocks[block].opcode[step];
    //         println!("{:?}", op);
    //     }
    //     let v: Rc<dyn Receiver> = match op {
    //         Operation::Int(v) => Rc::new(IntReceiver::new(*v)),
    //         Operation::Str(_) => todo!(),
    //         Operation::Invoke(selector, receiver, args) => {
    //             let receiver = self.get_value(receiver);
    //             let args = self.get_values(args);
    //             let r = receiver.receive_message(SelectorSet::get(selector), args);
    //             r
    //         }
    //         Operation::Block(b) => {
    //             let r = Rc::new(BlockContext::new(
    //                 CodeAddress(*b, 0),
    //                 method_context.clone(),
    //             ));
    //             r
    //         }
    //         Operation::Global(_) => todo!(),
    //         Operation::Char(v) => Rc::new(IntReceiver::new(*v as isize)),
    //         Operation::String(v) => Rc::new(StringReceiver::new(v.clone())),
    //         Operation::Arg(_) => todo!(),
    //         Operation::Return(_) => todo!(),
    //         Operation::Param(_) => todo!(),
    //         Operation::Myself => todo!(),
    //     };
    //     {
    //         let mut ip = self.ip.try_lock().unwrap();
    //         self.values.try_lock().unwrap().insert(*ip, v);
    //         // if step + 1 < self.method.blocks[block].opcode.len() {
    //         *ip = CodeAddress(block, step + 1);
    //         // }
    //     }
    // }

    fn result(&self, idx: usize) -> Rc<dyn Receiver> {
        // match self.method.blocks[idx].result {
        //     Some(addr) => self.get_value(&addr),
        //     None => NilReciever::get(),
        // }
        todo!()
    }

    fn get_values(&self, args: &Vec<CodeAddress>) -> Vec<Rc<dyn Receiver>> {
        let values = self.values.try_lock().unwrap();
        args.iter()
            .map(|x| values.get(x).unwrap().clone())
            .collect::<Vec<_>>()
    }

    fn get_value(&self, receiver: &CodeAddress) -> Rc<dyn Receiver> {
        let values = self.values.try_lock().unwrap();
        values.get(receiver).unwrap().clone()
    }

    fn done(&self) -> bool {
        // let CodeAddress(block, idx) = self.instruction_pointer.try_lock().unwrap().clone();
        // let blk = &self.method.blocks[block];
        // blk.opcode.len() <= idx
        todo!()
    }

    fn set_ip(&self, start: CodeAddress) -> CodeAddress {
        // let mut ip = self.ip.try_lock().unwrap();
        // let r = ip.clone();
        // *ip = start;
        // r
        CodeAddress(0, 0)
    }
}

impl BlockContext {
    pub fn new(ctx: ContextRef) -> Rc<Self> {
        let r = Self {
            // start: CodeAddress(0, 0),
            parent: ctx,
            // params: Mutex::new(vec![]),
        };
        let rr = Rc::new(r);
        rr
    }
}

pub fn parse_method(
    input_string: String,
) -> Result<Vec<Rc<Tree<AST>>>, Box<dyn std::error::Error>> {
    let lexing_rules = parser::lexer_rules();
    let grammar = parser::grammar();
    let mut lexemes = handle_lex_error(lex(&lexing_rules, &input_string))?;
    lexemes.insert(
        0,
        Rc::new(Lexeme {
            kind: "METHOD".into(),
            raw: String::new(),
            position: Position { line: 0, column: 0 },
        }),
    );
    let parse_trees = handle_error(parse(&grammar, &lexemes))?;
    Ok(parse_trees)
}

pub fn parse_script(
    input_string: String,
) -> Result<Vec<Rc<Tree<AST>>>, Box<dyn std::error::Error>> {
    let lexing_rules = parser::lexer_rules();
    let grammar = parser::grammar();
    let mut lexemes = handle_lex_error(lex(&lexing_rules, &input_string))?;
    lexemes.insert(
        0,
        Rc::new(Lexeme {
            kind: "EVALUATE".into(),
            raw: String::new(),
            position: Position { line: 0, column: 0 },
        }),
    );
    let parse_trees = handle_error(parse(&grammar, &lexemes))?;
    Ok(parse_trees)
}

#[allow(dead_code)]
impl Context {
    fn new(myself: Rc<dyn Receiver>) -> Self {
        Self {
            receiver_names: Mutex::new(BTreeMap::new()),
            myself,
        }
    }

    pub fn set_receiver(&self, name: &'static str, rec: Rc<dyn Receiver>) {
        let mut map = self.receiver_names.try_lock().unwrap();
        map.insert(name, rec);
    }

    pub fn get_receiver(&self, name: &'static str) -> Option<Rc<dyn Receiver>> {
        let map = self.receiver_names.try_lock().unwrap();
        match map.get(name) {
            Some(r) => Some(r.clone()),
            None => None,
        }
    }

    fn eval(&mut self, t: &AST) -> ObjectPtr {
        match t {
            AST::Int(n) => Object::new_string(n.to_string().as_str()),
            AST::String(_) => todo!(),
            AST::Name(_) => todo!(),
            AST::Method {
                name: _,
                params: _,
                temps: _,
                body: _,
            } => todo!(),
            AST::Return(_) => todo!(),
            AST::PatternPart(_, _, _) => todo!(),
            AST::List(_, _) => todo!(),
            AST::Table(t) => {
                panic!("eval {:?}", t);
            }
            AST::Message { name: _, args: _ } => todo!(),
            AST::Variable(_) => todo!(),
            AST::Empty => todo!(),
            AST::Statements(s) => {
                let mut r = Object::new_string("<none>".into());
                for x in s {
                    r = self.eval(x);
                }
                r
            }
            AST::InvokeSequence(target, msgs) => {
                let target_obj = self.eval(target);
                let mut r = Object::new_string("<nomsg>".into());
                for m in msgs {
                    if let AST::Message { name, args } = m {
                        let mut oargs = vec![];
                        for v in args {
                            oargs.push(self.eval(v));
                        }
                        r = target_obj.send(name, oargs.as_slice());
                    }
                }
                r
            }
            _ => todo!("{:?}", t),
        }
    }

    fn eval_to_reciever(&mut self, t: &AST) -> Rc<dyn runtime::Receiver> {
        match t {
            AST::Int(n) => Rc::new(IntReceiver::new(*n)),
            AST::String(s) => Rc::new(StringReceiver::new(String::from(*s))),
            AST::Name(_) => todo!(),
            AST::Method {
                name: _,
                params: _,
                temps: _,
                body: _,
            } => todo!(),
            AST::Return(x) => self.eval_to_reciever(x),
            AST::PatternPart(_, _, _) => todo!(),
            AST::List(_, _) => todo!(),
            AST::Table(t) => {
                let v: Vec<Rc<dyn Receiver>> = t.iter().map(|x| self.eval_to_reciever(x)).collect();
                Rc::new(ArrayReceiver(v))
            }
            AST::Message { name: _, args: _ } => todo!(),
            AST::Variable(name) => {
                if let Some(r) = self.get_receiver(*name) {
                    r
                } else {
                    match *name {
                        "self" => self.myself.clone(),
                        "Point" => Rc::new(PointMetaReceiver),
                        "Integer" => Rc::new(IntMetaReceiver),
                        _ => todo!("name not known: {}", name),
                    }
                }
            }
            AST::Empty => todo!(),
            AST::Statements(s) => {
                let mut r = NilReciever::get();
                for x in s {
                    r = self.eval_to_reciever(x);
                }
                r
            }
            AST::InvokeSequence(target, msgs) => {
                let mut receiver = self.eval_to_reciever(target);
                for m in msgs {
                    if let AST::Message { name, args } = m {
                        let mut oargs = vec![];
                        for v in args {
                            match v {
                                AST::Empty => panic!("{:#?}", args),
                                _ => {
                                    let o = self.eval_to_reciever(v);
                                    oargs.push(o);
                                }
                            }
                        }
                        receiver = receiver.receive_message(name, oargs);
                    }
                }
                receiver
            }
            AST::Assign(name, expr) => {
                if let AST::Name(name) = **name {
                    let value = self.eval_to_reciever(expr);
                    self.set_receiver(SelectorSet::get(name), value.clone());
                    value
                } else {
                    panic!("unexpected {:?}", t)
                }
            }
            AST::Block {
                params,
                temps,
                body,
            } => {
                info!("instantiate block");
                let r = BlockReceiver::new(self.myself.clone(), params, temps, body.clone());
                let map = self.receiver_names.try_lock().unwrap();
                for (k, v) in map.iter() {
                    info!("push {} to block context", *k);
                    r.define(*k, v.clone());
                }
                Rc::new(r)
            }
            AST::Char(c) => Rc::new(CharReceiver::new(*c)),
            _ => todo!("{:?}", t),
        }
    }
}

type Lexemes = Vec<Rc<Lexeme>>;

fn handle_lex_error(r: Result<Lexemes, LexerError>) -> Result<Lexemes, AppError> {
    match r {
        Ok(l) => Ok(l),
        Err(e) => Err(AppError { msg: Box::new(e) }),
    }
}

type ParseTrees = Vec<Rc<Tree<AST>>>;

fn handle_error(r: Result<ParseTrees, ParseError<AST>>) -> Result<ParseTrees, AppError> {
    match r {
        Ok(t) => Ok(t),
        Err(e) => Err(AppError { msg: Box::new(e) }),
    }
}

#[allow(dead_code)]
fn handle_parse_tree(parse_trees: Vec<Rc<Tree<AST>>>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Parse Trees:");
    for tree in &parse_trees {
        println!("{tree}");
        let ast = tree.as_abstract_syntax_tree();
        println!("Abstract Syntax Tree:");
        println!("{ast:#?}");
    }
    // println!("Evaluated:");
    // println!("{}", eval(&ast));
    Ok(())
}

#[allow(dead_code)]
fn print_lexemes(lexemes: &Vec<Rc<Lexeme>>) {
    println!("Lexemes:");
    for lexeme in lexemes {
        println!("  {lexeme}");
    }
}
