use santiago::lexer::LexerRules;

pub fn lexer_rules() -> LexerRules {
    santiago::lexer_rules!(
        "DEFAULT" | "INT" = pattern r"[0-9]+";
        "DEFAULT" | "IDENTIFIER" = pattern r"[a-zA-Z_][a-zA-Z_0-9]*";
        "DEFAULT" | "KEYWORD" = pattern r"[a-zA-Z_][a-zA-Z_0-9]*:";
        // "DEFAULT" | "LOCAL" = pattern r":[a-zA-Z_][a-zA-Z_0-9]*";
        "DEFAULT" | "COMMENT" = pattern "\"[^\"]*\"" => |l| l.skip();
        "DEFAULT" | ":" = string ":";
        // "DEFAULT" | "BINARY" = string "+";
        // "DEFAULT" | "BINARY" = string "-";
        // "DEFAULT" | "BINARY" = string "*";
        // "DEFAULT" | "/" = string "/";
        "DEFAULT" | "END_OF_CHUNK" = string "!";
        "DEFAULT" | "^" = string "^";
        "DEFAULT" | "." = string ".";
        "DEFAULT" | "[" = string "[";
        "DEFAULT" | "]" = string "]";
        "DEFAULT" | "(" = string "(";
        "DEFAULT" | ")" = string ")";
        "DEFAULT" | "|" = string "|";
        "DEFAULT" | "ASSIGN" = string ":=";
        "DEFAULT" | "BINARY" = pattern r"[=+\-*/]+";
        "DEFAULT" | "CHAR" = pattern r"\$.";
        "DEFAULT" | "WS" = pattern r"\s" => |lexer| lexer.skip();
    )
}

// use santiago::grammar::Associativity;
use santiago::grammar::Grammar;

use crate::runtime::get_selector;

#[derive(Debug, Clone)]
pub enum AST {
    Int(isize),
    String(String),
    Name(String),
    Method {
        name: &'static str,
        params: Vec<String>,
        temps: Vec<String>,
        body: Box<AST>,
    },
    Return(Box<AST>),
    PatternPart(String, Option<Box<AST>>, Box<AST>),
    List(Box<AST>, Box<AST>),
    Table(Vec<Box<AST>>),
    Statements(Vec<AST>),
    Messages(Box<AST>, Vec<AST>),
    Message {
        name: &'static str,
        args: Vec<AST>,
    },
    Variable(String),
    Empty,
}

impl From<&AST> for String {
    fn from(s: &AST) -> Self {
        println!("AST to String: {:?}", s);
        match s {
            AST::Empty => String::from("<empty>"),
            AST::Name(x) => x.clone(),
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

fn params_from(name: &AST) -> Vec<String> {
    match name {
        AST::PatternPart(_, Some(x0), rest) => match *x0.clone() {
            AST::Name(x) => {
                let mut start = vec![x.clone()];
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
            get_selector(format!("{}{}", x, &r).as_str())
        }
        AST::Empty => "",
        AST::Name(s) => get_selector(s.as_str()),
        _ => {
            println!("selector_from {:?}", name);
            unreachable!()
        }
    }
}

fn table_add(a: &AST, b: &AST) -> AST {
    if let AST::Table(t) = a {
        let mut t_new = t.clone();
        t_new.push(Box::new(b.clone()));
        AST::Table(t_new)
    } else {
        unreachable!()
    }
}

fn table_from(n: &AST) -> AST {
    AST::Table(vec![Box::new(n.clone())])
}

pub fn grammar() -> Grammar<AST> {
    santiago::grammar!(
        "cmd" => rules "define_cmd" "def";
        "cmd" => rules "eval_cmd" "statements" "dot" => |r:Vec<AST>| r[1].clone();

        "def" => rules "method definition";
        "def" => empty => |_| AST::Empty;

        "method definition" => rules "message pattern" "temporaries" "statements"
            => |r| gen_method(&r[0], &r[1], &r[2]);

        "chunk sep" => lexemes "END_OF_CHUNK" => |_| AST::Empty;
        "temporaries" => empty  => |_| AST::Empty;
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
        "statements" => empty => |_| AST::Statements(vec![]);
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
        "statements" => rules "expression" => |r| AST::Statements(vec![r[0].clone()]);
        "return statement" => rules "return op" "expression"
            => |r| AST::Return(Box::new(r[1].clone()));
        "expression" => rules "basic expression" => |r| r[0].clone();
        "expression" => rules "assignment" => |r| r[0].clone();
        "assignment" => rules "identifier" "assignmentOperator" "expression" => |r| r[0].clone();
        "basic expression" => rules "primary" => |r| r[0].clone();
        "basic expression" => rules "primary" "messages"
                => |r| if let AST::Messages(_, msgs) = &r[1] {
                    AST::Messages(Box::new(r[0].clone()), msgs.clone())
                }
                else {
                    panic!("sub tree is not a messages list.")
                };
        "messages" => rules "keyword message or empty"
                    =>|r| r[0].clone();
        "unary messages or empty" => rules "unary messages" => |r| r[0].clone();
        "unary messages or empty" => empty => |_| AST::Empty;
        "unary messages" => rules "unary message" => |r| r[0].clone();
        "unary message" => rules "unarySelector" => |r|     AST::Message {  name: selector_from(&r[0]),
                                                                            args: vec![] };
        "binary messages or empty" => rules "binary messages" => |r| r[0].clone();
        "binary messages or empty" => empty => |_| AST::Empty;
        "binary messages" => rules "binary message" => |r| r[0].clone();
        "binary message" => rules "unary message"
            => |r| r[0].clone();
        "binary message" => rules "unary messages" "binarySelector" "expression"
            => |r| AST::Message{name: selector_from(&r[1]),
                                args: vec![r[2].clone()]};
        "binary message" => rules "binarySelector" "expression"
            => |r| AST::Messages(Box::new(AST::Empty), vec![AST::Message{
                                name: selector_from(&r[0]),
                                args: vec![r[1].clone()]}]);
        // "keyword message or empty" => rules "binary messages"
        //     => |r| r[0].clone();
        "keyword message or empty" => rules "binary message"
            => |r| r[0].clone();
        "keyword message or empty" => rules "keyword message"
            => |r| r[0].clone();
        "keyword message or empty" => rules "binary message" "keyword message parts"
            => |r|   AST::Message { name: selector_from(&r[1]),
                args: args_from(&r[1]) };
        "keyword message" => rules "keyword message parts"
            => |r|   AST::Message { name: selector_from(&r[0]),
                                    args: args_from(&r[0]) };

        "keyword message parts" => rules "keyword" "keyword argument" "keyword message parts"
            => |r| AST::PatternPart(String::from(&r[0]),
                                    Some(r[1].clone().into()),
                                    Box::new(r[2].clone()));
        "keyword message parts" => rules "keyword" "keyword argument"
            => |r| AST::PatternPart(String::from(&r[0]),
            Some(r[1].clone().into()),
            Box::new(AST::Empty));
        "keyword argument" => rules "primary" "unary messages or empty" "binary messages or empty"
            => |r| r[0].clone();
        "primary" => lexemes "IDENTIFIER" => |l| AST::Variable(String::from(&l[0].raw));
        "primary" => lexemes "CHAR" => |l| AST::String(String::from(&l[0].raw));
        "primary" => lexemes "INT" => |l| AST::Int(l[0].raw.to_string().parse::<isize>().unwrap());
        "primary" => rules "block constructor" => |r| r[0].clone();
        "primary" => rules "openParen" "expression" "closeParen" => |r| r[1].clone();
        "block constructor" => rules "blockStart" "block arguments" "temporaries" "block body" "blockEnd" => |r| r[0].clone();
        "block arguments" => rules "colon" "identifier" "bar" => |r| r[0].clone();
        "block arguments" => empty => |_| AST::Empty;
        "block body" => rules "statements" => |r| r[0].clone();
        "dot" => lexemes "." => |_| AST::Empty;
        "return op" => lexemes "^" => |_| AST::Empty;
        "unarySelector" => lexemes "IDENTIFIER" => |l| AST::Name(String::from(&l[0].raw));
        "identifier" => lexemes "IDENTIFIER" => |l| AST::Name(String::from(&l[0].raw));
        "binarySelector" => lexemes "BINARY" => |l| AST::Name(String::from(&l[0].raw));
        "assignmentOperator" => lexemes "ASSIGN" => |_| AST::Empty;
        "keyword" => lexemes "KEYWORD" => |l| AST::Name(String::from(&l[0].raw));
        "blockStart" => lexemes "[" => |_| AST::Empty;
        "blockEnd" => lexemes "]" => |_| AST::Empty;
        "colon" => lexemes ":" => |_| AST::Empty;
        "bar" => lexemes "|" => |_| AST::Empty;
        "openParen" => lexemes "(" => |_| AST::Empty;
        "closeParen" => lexemes ")" => |_| AST::Empty;
        "define_cmd" => lexemes "DEFINE" => |_| AST::Empty;
        "eval_cmd" => lexemes "EVALUATE" => |_| AST::Empty;
    )
}

pub fn eval(value: &AST) -> isize {
    match value {
        AST::Int(int) => *int,
        _ => unreachable!(),
    }
}
