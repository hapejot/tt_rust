


use rustyline::{
    error::ReadlineError,
    validate::{ValidationContext, ValidationResult},
    Completer, Helper, Highlighter, Hinter,
};

use tt_rust::evaluate_script;



#[derive(Completer, Helper, Highlighter, Hinter)]
struct InputValidator {}

impl rustyline::validate::Validator for InputValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult, ReadlineError> {
        use ValidationResult::{Incomplete, Valid};
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
    let mut rl = rustyline::Editor::with_config(
        rustyline::Config::builder()
            // .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Vi)
            .build(),
    )?;
    rl.set_helper(Some(InputValidator {}));

    let input_string = rl.readline("> ")?;

    evaluate_script(input_string)?;
    Ok(())
}

