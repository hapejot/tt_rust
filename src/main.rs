use santiago::parser::ParseError;
use tt_rust::parser::{lexer_rules, grammar, AST};


fn main() -> Result<(), ()> {
    use std::io::Read;

    let lexing_rules = lexer_rules();
    let grammar = grammar();

    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin).unwrap();

    match santiago::lexer::lex(&lexing_rules, &stdin) {
        Ok(lexemes) => {
            print_lexemes(&lexemes);

            match santiago::parser::parse(&grammar, &lexemes) {
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

fn handle_lex_error(error: santiago::lexer::LexerError) -> Result<(), ()> {
    println!("Lexing Error:");
    println!("{}",error);
    Err(())
}

fn handle_error(error: ParseError<AST>) -> Result<(), ()> {
    println!("Parsing Error:");
    println!("{error}");
    Err(())
}

fn handle_parse_tree(parse_trees: Vec<std::rc::Rc<santiago::parser::Tree<tt_rust::parser::AST>>>) -> Result<(), ()> {
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

fn print_lexemes(lexemes: &Vec<std::rc::Rc<santiago::lexer::Lexeme>>) {
    println!("Lexemes:");
    for lexeme in lexemes {
        println!("  {lexeme}");
    }
}
