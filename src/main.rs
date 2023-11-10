use std::{fmt::Display, io::stdin, rc::Rc};

use crossterm::cursor::DisableBlinking;
use rustyline::{
    error::ReadlineError,
    validate::{ValidationContext, ValidationResult},
    Completer, Helper, Highlighter, Hinter,
};
use santiago::{
    lexer::{lex, Lexeme, LexerError, Position},
    parser::{parse, ParseError, Tree},
};
use tt_rust::{parser::{grammar, lexer_rules, AST}, runtime::Object};

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

#[derive(Completer, Helper, Highlighter, Hinter)]
struct InputValidator {}

impl rustyline::validate::Validator for InputValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult, ReadlineError> {
        use ValidationResult::{Incomplete, Invalid, Valid};
        let input = ctx.input();
        let result = if !input.ends_with('.') {
            Incomplete
        } else {
            Valid(None)
        };
        Ok(result)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lexing_rules = lexer_rules();
    let grammar = grammar();

    let mut rl = rustyline::Editor::with_config(
        rustyline::Config::builder()
            // .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Vi)
            .build(),
    )?;
    rl.set_helper(Some(InputValidator {}));

    let input_string = rl.readline("> ")?;

    let mut lexemes = handle_lex_error(lex(&lexing_rules, &input_string))?;
    lexemes.insert(
        0,
        Rc::new(Lexeme {
            kind: "EVALUATE".into(),
            raw: String::new(),
            position: Position { line: 0, column: 0 },
        }),
    );
    print_lexemes(&lexemes);
    let parse_trees = handle_error(parse(&grammar, &lexemes))?;
    let mut ctx = Context::new();
    for t in parse_trees {
        let ast = t.as_abstract_syntax_tree();
        // println!("-> {}", t.to_string());
        println!("eval -> {}", ctx.eval(&ast));
    }
    Ok(())
}



struct Context {}

impl Context {
    fn new() -> Self {
        Self {}
    }

    fn eval(&mut self, t: &AST) -> Object {
        match t {
            AST::Int(n) => Object::new(n.to_string()),
            AST::String(_) => todo!(),
            AST::Name(_) => todo!(),
            AST::Method {
                name,
                params,
                temps,
                body,
            } => todo!(),
            AST::Return(_) => todo!(),
            AST::PatternPart(_, _, _) => todo!(),
            AST::List(_, _) => todo!(),
            AST::Table(t) => {
                panic!("eval {:?}", t);
            }
            AST::Message { name, args } => todo!(),
            AST::Variable(_) => todo!(),
            AST::Empty => todo!(),
            AST::Statements(s) => {
                let mut r = Object::new("<none>".into());
                for x in s {
                    r = self.eval(x);
                }
                r
            }
            AST::Messages(target, msgs) => {
                let target_obj = self.eval(target);
                let mut r = Object::new("<nomsg>".into());
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

fn print_lexemes(lexemes: &Vec<Rc<Lexeme>>) {
    println!("Lexemes:");
    for lexeme in lexemes {
        println!("  {lexeme}");
    }
}
