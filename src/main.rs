use santiago::lexer::LexerRules;

pub fn lexer_rules() -> LexerRules {
    santiago::lexer_rules!(
        "DEFAULT" | "INT" = pattern r"[0-9]+";
        "DEFAULT" | "IDENTIFIER" = pattern r"[a-zA-Z_][a-zA-Z_0-9]*";
        "DEFAULT" | "+" = string "+";
        "DEFAULT" | "-" = string "-";
        "DEFAULT" | "*" = string "*";
        "DEFAULT" | "/" = string "/";
        "DEFAULT" | "^" = string "^";
        "DEFAULT" | "." = string ".";
        "DEFAULT" | "WS" = pattern r"\s" => |lexer| lexer.skip();
    )
}

use santiago::grammar::Associativity;
use santiago::grammar::Grammar;

#[derive(Debug)]
pub enum AST {
    Int(isize),
    BinaryOperation(Vec<AST>),
    OperatorAdd,
    OperatorSubtract,
    OperatorMultiply,
    OperatorDivide,
}

pub fn grammar() -> Grammar<AST> {
    santiago::grammar!(
        "statements" => rules "return-statement";
        "statements" => rules "return-statement" "dot";
        "return-statement" => rules "return-op" "expression";
        "expression" => rules "basic-expression";
        "basic-expression" => rules "primary";
        "primary" => lexemes "IDENTIFIER";
        "dot" => lexemes ".";
        "return-op" => lexemes "^";
    )
}

pub fn eval(value: &AST) -> isize {
    match value {
        AST::Int(int) => *int,
        AST::BinaryOperation(args) => match &args[1] {
            AST::OperatorAdd => eval(&args[0]) + eval(&args[2]),
            AST::OperatorSubtract => eval(&args[0]) - eval(&args[2]),
            AST::OperatorMultiply => eval(&args[0]) * eval(&args[2]),
            AST::OperatorDivide => eval(&args[0]) / eval(&args[2]),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

fn main() -> Result<(), ()> {
    use std::io::Read;

    let lexing_rules = lexer_rules();
    let grammar = grammar();

    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin).unwrap();

    match santiago::lexer::lex(&lexing_rules, &stdin) {
        Ok(lexemes) => {
            println!("Lexemes:");
            for lexeme in &lexemes {
                println!("  {lexeme}");
            }

            match santiago::parser::parse(&grammar, &lexemes) {
                Ok(parse_trees) => {
                    println!("Parse Trees:");
                    let parse_tree = &parse_trees[0];
                    println!("{parse_tree}");

                    // let ast = parse_tree.as_abstract_syntax_tree();

                    // println!("Abstract Syntax Tree:");
                    // println!("{ast:#?}");

                    // println!("Evaluated:");
                    // println!("{}", eval(&ast));

                    Ok(())
                }
                Err(error) => {
                    println!("Parsing Error:");
                    println!("{error}");
                    Err(())
                }
            }
        }
        Err(error) => {
            println!("Lexing Error:");
            println!("{error}");
            Err(())
        }
    }
}
