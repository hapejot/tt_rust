use std::{collections::BTreeMap, fmt::Display, rc::Rc};

use parser::AST;
use runtime::{
    int::IntReceiver, nil::NilReciever, pnt::PointMetaReceiver, sel::SelectorSet,
    str::StringReceiver, Object, ObjectPtr, Receiver, blk::BlockReceiver,
};

use santiago::{
    lexer::{lex, Lexeme, LexerError, Position},
    parser::{parse, ParseError, Tree},
};
use std::{path::Path, sync::Mutex};

use once_cell::sync::Lazy;

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

struct Context {
    receiver_names: Mutex<BTreeMap<&'static str, Rc<dyn Receiver>>>,
    myself: Rc<dyn Receiver>,
}

#[allow(dead_code)]
impl Context {
    fn new(myself: Rc<dyn Receiver>) -> Self {
        Self {
            receiver_names: Mutex::new(BTreeMap::new()),
            myself,
        }
    }

    fn set_receiver(&self, name: &'static str, rec: Rc<dyn Receiver>) {
        let mut map = self.receiver_names.lock().unwrap();
        map.insert(name, rec);
    }

    fn get_receiver(&self, name: &'static str) -> Option<Rc<dyn Receiver>> {
        let map = self.receiver_names.lock().unwrap();
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
            AST::String(s) => Rc::new(StringReceiver(String::from(*s))),
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
                panic!("eval {:?}", t);
            }
            AST::Message { name: _, args: _ } => todo!(),
            AST::Variable(name) => {
                if let Some(r) = self.get_receiver(*name) {
                    r
                } else {
                    match *name {
                        "self" => self.myself.clone(),
                        "Point" => Rc::new(PointMetaReceiver),
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
                        receiver = receiver.receive_message(name, oargs.as_slice());
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
            AST::Block { params, temps, body } => {
                Rc::new(BlockReceiver::new(params, temps, body.clone()))
            }
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
