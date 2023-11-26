use santiago::lexer::LexerRules;
use tracing::{error, info};

pub fn lexer_rules() -> LexerRules {
    santiago::lexer_rules!(
        "DEFAULT" | "INT" = pattern r"[0-9]+";
        "DEFAULT" | "IDENTIFIER" = pattern r"[a-zA-Z_][a-zA-Z_0-9]*";
        "DEFAULT" | "KEYWORD" = pattern r"[a-zA-Z_][a-zA-Z_0-9]*:";
        "DEFAULT" | "STRING" = pattern r"'[^']*'";
         // "DEFAULT" | "LOCAL" = pattern r":[a-zA-Z_][a-zA-Z_0-9]*";
        "DEFAULT" | "COMMENT" = pattern "\"[^\"]*\"" => |l| l.skip();
        "DEFAULT" | ":" = string ":";
        "DEFAULT" | "END_OF_CHUNK" = string "!";
        "DEFAULT" | "." = string ".";
        "DEFAULT" | "[" = string "[";
        "DEFAULT" | "]" = string "]";
        "DEFAULT" | "(" = string "(";
        "DEFAULT" | ")" = string ")";
        "DEFAULT" | "|" = string "|";
        "DEFAULT" | "{" = string "{";
        "DEFAULT" | "}" = string "}";
        "DEFAULT" | "ASSIGN" = string ":=";
        "DEFAULT" | "ASSIGN" = string "<-";
        "DEFAULT" | "BINARY" = pattern r"[-%&,*+/<=>?@\~!]+";
        "DEFAULT" | "CHAR" = pattern r"\$.";
        "DEFAULT" | "WS" = pattern r"\s" => |lexer| lexer.skip();
        "DEFAULT" | "RETURN" = string "^";
    )
}

// use santiago::grammar::Associativity;
use santiago::grammar::Grammar;

use crate::runtime::sel::SelectorSet;

#[derive(Debug, Clone)]
pub enum AST {
    Int(isize),
    Char(char),
    String(&'static str),
    Name(&'static str),
    Method {
        name: &'static str,
        params: Vec<&'static str>,
        temps: Vec<&'static str>,
        body: Box<AST>,
    },
    Block {
        params: Vec<&'static str>,
        temps: Vec<&'static str>,
        body: Box<AST>,
    },
    Return(Box<AST>),
    PatternPart(String, Option<Box<AST>>, Box<AST>),
    List(Box<AST>, Box<AST>),
    Table(Vec<Box<AST>>),
    Statements(Vec<AST>),
    InvokeSequence(Box<AST>, Vec<AST>),
    InvokeCascade(Box<AST>, Vec<AST>),
    Message {
        name: &'static str,
        args: Vec<AST>,
    },
    Variable(&'static str),
    Assign(Box<AST>, Box<AST>),
    Dummy(String),
    Empty,
}

impl From<&AST> for String {
    fn from(s: &AST) -> Self {
        println!("AST to String: {:?}", s);
        match s {
            AST::Empty => String::from("<empty>"),
            AST::Name(x) => (*x).into(),
            _ => format!("{:?}", s),
        }
    }
}

fn gen_method(name: &AST, _temps: &AST, body: &AST) -> AST {
    AST::Method {
        name: selector_from(name),
        params: params_from(name),
        temps: vec![],
        body: Box::new(body.clone()),
    }
}

fn names_from(t: &AST) -> Vec<&'static str> {
    match t {
        AST::Table(tab) => {
            let mut r = vec![];
            for x in tab {
                match **x {
                    AST::Name(n) => r.push(n),
                    _ => panic!("unexpected"),
                }
            }
            r
        }
        _ => panic!("not expected"),
    }
}

fn params_from(name: &AST) -> Vec<&'static str> {
    match name {
        AST::PatternPart(_, Some(x0), rest) => match *x0.clone() {
            AST::Name(x) => {
                let mut start = vec![x];
                start.extend_from_slice(params_from(rest).as_slice());
                start
            }
            _ => unreachable!(),
        },
        AST::Empty => vec![],
        AST::PatternPart(_, None, _) => vec![],
        _ => {
            println!("prams_from {:?}", name);
            unreachable!()
        }
    }
}

fn args_from(name: &AST) -> Vec<AST> {
    match name {
        AST::PatternPart(_, Some(x), rest) => {
            let mut start: Vec<AST> = vec![*x.clone()];
            start.extend_from_slice(args_from(rest).as_slice());
            start
        }
        AST::Empty => vec![],
        AST::PatternPart(_, None, _) => vec![],
        _ => {
            println!("prams_from {:?}", name);
            unreachable!()
        }
    }
}

fn selector_from(name: &AST) -> &'static str {
    match name {
        AST::PatternPart(x, _, rest) => {
            let r = selector_from(rest);
            SelectorSet::get(format!("{}{}", x, &r).as_str())
        }
        AST::Empty => "",
        AST::Name(s) => SelectorSet::get(*s),
        _ => {
            println!("selector_from {:?}", name);
            unreachable!()
        }
    }
}

#[allow(dead_code)]
fn table_add(a: &AST, b: &AST) -> AST {
    if let AST::Table(t) = a {
        let mut t_new = t.clone();
        t_new.push(Box::new(b.clone()));
        AST::Table(t_new)
    } else {
        unreachable!()
    }
}

#[allow(dead_code)]
fn table_from(n: &AST) -> AST {
    AST::Table(vec![Box::new(n.clone())])
}

pub fn grammar() -> Grammar<AST> {
    santiago::grammar!(
        "cmd" => rules "define_cmd" "def";
        "cmd" => rules "method_cmd" "method definition" => |r:Vec<AST>| r[1].clone();
        "cmd" => rules "eval_cmd" "statements" => |r:Vec<AST>| r[1].clone();
        "cmd" => rules "eval_cmd" "statements" "dot"=> |r:Vec<AST>| r[1].clone();

        "def" => rules "method definition";
        "def" => empty => |_| AST::Empty;

        "method definition" => rules "message pattern" "temporaries" "statements"
            => |r| gen_method(&r[0], &r[1], &r[2]);

        "chunk sep" => lexemes "END_OF_CHUNK" => |_| AST::Dummy(String::from("chunk separator"));
        "temporaries" => empty  => |_| AST::Dummy(String::from("temporaries"));
        "temporaries" => rules "bar" "identifiers" "bar" => |r| r[1].clone();
        "identifiers" => rules "identifier" => |r| r[0].clone();
        "identifiers" => rules "identifiers" "identifier" => |r| r[0].clone();
        "message pattern" => rules "unary pattern" => |r| r[0].clone();
        "message pattern" => rules "binary pattern" => |r| r[0].clone();
        "message pattern" => rules "keyword pattern" => |r| r[0].clone();
        "unary pattern" => rules "unarySelector"
            => |r| AST::PatternPart((&r[0]).into(), None, Box::new(AST::Empty));
        "binary pattern" => rules "binarySelector" "identifier"
            => |r| AST::PatternPart((&r[0]).into(), Some(r[1].clone().into()), Box::new(AST::Empty));
        "keyword pattern" => rules "keyword"  "identifier"
            => |r| AST::PatternPart((&r[0]).into(), Some(r[1].clone().into()), Box::new(AST::Empty));
        "keyword pattern" => rules "keyword"  "identifier" "keyword pattern"
            => |r| AST::PatternPart((&r[0]).into(), Some(r[1].clone().into()), Box::new(r[2].clone()));
        "statements" => rules "expression"
            => |r| {  AST::Statements(vec![r[0].clone()])  };
        "statements" => rules "return statement" => |r| AST::Statements(vec![r[0].clone()]);
        "statements" => rules "return statement" "dot" => |r| AST::Statements(vec![r[0].clone()]);
        "statements" => rules "expression" "dot" "statements"
            => |r| {
            if let AST::Statements(x) = &r[2]{
                let mut v = vec![r[0].clone()];
                for e in x {v.push(e.clone());}
                AST::Statements(v)
            }
            else {
                AST::Statements(vec![r[0].clone()])
            }
        };
        // "statements" => rules "expression" => |r| AST::Statements(vec![r[0].clone()]);
        "return statement" => rules "return op" "expression"
            => |r| AST::Return(Box::new(r[1].clone()));
        "expression" => rules "basic expression" => |r| r[0].clone();
        "expression" => rules "assignment" => |r| r[0].clone();
        "assignment" => rules "identifier" "assignmentOperator" "expression"
            => |r| AST::Assign(Box::new(r[0].clone()), Box::new(r[2].clone()));
        "basic expression" => rules "primary" => |r| r[0].clone();
        "basic expression" => rules "primary" "messages"
                => |r| if let AST::InvokeSequence(_, msgs) = &r[1] {
                    AST::InvokeSequence(Box::new(r[0].clone()), msgs.clone())
                }
                else {
                    error!("sub tree is not a messages list. {:#?}", r);
                    r[0].clone()
                };

        "binary messages" => rules "unary messages"
            => |r| r[0].clone();
        "binary messages" => rules "binary messages" "binary message"
            => |r| {
            if let AST::InvokeSequence(target, msgs) = &r[0] {
                let mut ms = msgs.clone();
                ms.push(r[1].clone());
                AST::InvokeSequence(target.clone(), ms)
            }
            else {
                panic!()
                // AST::Dummy(String::from("no invoke seq"))
            }
        };


        "messages" => rules "binary messages" "keyword message"
            => |r| {
                if let AST::InvokeSequence(target, msgs) = &r[0] {
                    let mut ms = msgs.clone();
                    ms.push(r[1].clone());
                    AST::InvokeSequence(target.clone(), ms)
                }
                else {
                    panic!()
                    // AST::Dummy(String::from("no invoke seq"))
                }
            };
        "messages" => rules "binary messages"
            => |r| r[0].clone();
        "unary messages" => empty
            =>|_| AST::InvokeSequence(Box::new(AST::Empty), vec![]);
        "unary messages" => rules "unary messages" "unary message"
            => |r| {
                if let AST::InvokeSequence(t, v0) = &r[0]{
                    let mut v = v0.clone();
                    v.push(r[1].clone());
                    AST::InvokeSequence(t.clone(), v) }
                else {
                    panic!();
                }
        };

        "unary message" => rules "unarySelector"
            => |r|     AST::Message {  name: selector_from(&r[0]),
                                       args: vec![] };
        "unary expression" => rules "primary" "unary messages"
            => |r| match &r[1] {
                        AST::InvokeSequence(_,seq) =>   if seq.len() == 0 {
                                                            r[0].clone()
                                                        }
                                                        else {
                                                            let result = AST::InvokeSequence(Box::new(r[0].clone()), seq.clone());
                                                            result
                                                        },
                        _ => todo!(),
        };

        "binary message" => rules "binarySelector" "unary expression" // "expression" is not working, since it would generate an implict right associated tree, which is wrong for Smalltalk
            => |r| AST::Message{name: selector_from(&r[0]),
                                args: vec![r[1].clone()]};

        "binary expression" => rules "unary expression"
            => |r| r[0].clone();
        "binary expression" => rules "binary expression" "binarySelector" "unary expression"
            => |r| match (&r[0], &r[1]) {
                    (AST::InvokeSequence(receiver, msgs), AST::Name(name)) => {
                        let mut ms = msgs.clone();
                        ms.push(AST::Message { name, args: vec![r[2].clone()] });
                        AST::InvokeSequence(receiver.clone(), ms )},
                    (receiver, AST::Name(name)) => {
                            let ms = vec![AST::Message { name, args: vec![r[2].clone()] }];
                            AST::InvokeSequence(Box::new(receiver.clone()), ms )},
                        _ => todo!("{:?}", &r),
            };

        "keyword message" => rules "keyword message parts"
            => |r|   AST::Message { name: selector_from(&r[0]),
                                    args: args_from(&r[0]) };

        "keyword message parts" => rules "keyword" "keyword argument" "keyword message parts"
            => |r| {
                info!("message part: {:?} {:?}", &r[0], &r[1]);
                AST::PatternPart(String::from(&r[0]),
                                    Some(r[1].clone().into()),
                                    Box::new(r[2].clone()))};
        "keyword message parts" => rules "keyword" "keyword argument"
            => |r| {
                info!("message part: {:?} {:?}", &r[0], &r[1]);
                AST::PatternPart(String::from(&r[0]),
                                    Some(r[1].clone().into()),
                                    Box::new(AST::Empty))};
        "keyword argument" => rules "binary expression"
            => |r| r[0].clone();
        "primary" => lexemes "STRING" => |l| {
            let s = &l[0].raw;
            let s0 = &s[1..s.len()-1];
            AST::String(SelectorSet::get(s0))};
        "primary" => lexemes "IDENTIFIER" => |l| AST::Variable(SelectorSet::get(&l[0].raw));
        "primary" => lexemes "CHAR" => |l| if let Some(c) = l[0].raw.chars().nth(1) {
                AST::Char(c)
            } else {
                AST::Empty
            };
        "primary" => lexemes "INT" => |l| AST::Int(l[0].raw.to_string().parse::<isize>().unwrap());
        "primary" => rules "block constructor" => |r| r[0].clone();
        "primary" => rules "openBrace" "expression" "closeBrace" => |r| AST::Table(vec![Box::new(r[1].clone())]);
        "primary" => rules "openParen" "expression" "closeParen" => |r| r[1].clone();
        "block constructor" => rules "blockStart" "block args" "temporaries" "block body" "blockEnd"
            => |r| AST::Block{
                            params: names_from(&r[1]),
                            temps: vec![],
                            body: Box::new(r[3].clone()) };
        "block args" => rules "block arguments" "bar"
            => |r| r[0].clone();
        "block args" => empty
            => |_| AST::Table(vec![]);
        "block arguments" => empty => |_| AST::Table(vec![]);
        "block arguments" => rules "block arguments" "colon" "identifier"
            => |r| {
                if let AST::Table(mut lst) = r[0].clone()
                {
                    lst.push(Box::new(r[2].clone()));
                    AST::Table(lst)
                }
                else {
                    panic!("no table")
                }
            };
        "block body" => rules "statements" => |r| r[0].clone();
        "dot" => lexemes "." => |_| AST::Empty;
        "return op" => lexemes "RETURN" => |_| AST::Empty;
        "unarySelector" => lexemes "IDENTIFIER" => |l| AST::Name(SelectorSet::get(&l[0].raw));
        "identifier" => lexemes "IDENTIFIER" => |l| AST::Name(SelectorSet::get(&l[0].raw));
        "binarySelector" => lexemes "BINARY" => |l| AST::Name(SelectorSet::get(&l[0].raw));
        "assignmentOperator" => lexemes "ASSIGN" => |_| AST::Empty;
        "keyword" => lexemes "KEYWORD" => |l| AST::Name(SelectorSet::get(&l[0].raw));
        "blockStart" => lexemes "[" => |_| AST::Empty;
        "blockEnd" => lexemes "]" => |_| AST::Empty;
        "colon" => lexemes ":" => |_| AST::Empty;
        "bar" => lexemes "|" => |_| AST::Empty;
        "openParen" => lexemes "(" => |_| AST::Empty;
        "closeParen" => lexemes ")" => |_| AST::Empty;
        "openBrace" => lexemes "{" => |_| AST::Empty;
        "closeBrace" => lexemes "}" => |_| AST::Empty;
        "define_cmd" => lexemes "DEFINE" => |_| AST::Empty;
        "eval_cmd" => lexemes "EVALUATE" => |_| AST::Empty;
        "method_cmd" => lexemes "METHOD" => |_| AST::Empty;
    )
}

pub fn eval(value: &AST) -> isize {
    match value {
        AST::Int(int) => *int,
        _ => unreachable!(),
    }
}
