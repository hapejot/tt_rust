use std::{rc::Rc, io::stdin};

use santiago::{parser::{ParseError, Tree, parse}, lexer::{Lexeme, LexerError, lex}};
use tt_rust::parser::{lexer_rules, grammar, AST};
use std::io::Read;


fn main() -> Result<(), ()> {

    let lexing_rules = lexer_rules();
    let grammar = grammar();

    let mut input_string = String::new();
    stdin().read_to_string(&mut input_string).unwrap();

    match lex(&lexing_rules, &input_string) {
        Ok(lexemes) => {
            // print_lexemes(&lexemes);

            match parse(&grammar, &lexemes) {
                Ok(parse_trees) => {
                    handle_parse_tree(parse_trees)
                }
                Err(error) => {
                    handle_error(error)
                }
            }
        }
        Err(error) => {
            handle_lex_error(error)
        }
    }
}

fn handle_lex_error(error: LexerError) -> Result<(), ()> {
    println!("Lexing Error:");
    println!("{}",error);
    Err(())
}

fn handle_error(error: ParseError<AST>) -> Result<(), ()> {
    println!("Parsing Error:");
    println!("{error}");
    Err(())
}

fn handle_parse_tree(parse_trees: Vec<Rc<Tree<AST>>>) -> Result<(), ()> {
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
