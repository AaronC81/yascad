use std::rc::Rc;

use manifold_rs::Manifold;

use yascad_backend::Interpreter;
use yascad_frontend::{Parser, tokenize};
pub use yascad_frontend::{InputSource, InputSourceOrigin, ParseError, TokenizeError};
pub use yascad_backend::RuntimeError;

#[derive(Debug, Clone)]
pub enum LangError {
    Tokenize(Vec<TokenizeError>),
    Parser(Vec<ParseError>),
    Runtime(RuntimeError),
}

pub fn build_model(source: InputSource) -> Result<Manifold, LangError> {
    let source = Rc::new(source);

    let (tokens, errors) = tokenize(source.clone());
    if !errors.is_empty() {
        return Err(LangError::Tokenize(errors))
    }

    let mut parser = Parser::new(source.clone(), tokens);
    let stmts = parser.parse_statements();

    if !parser.errors.is_empty() {
        return Err(LangError::Parser(parser.errors))
    }

    let mut interpreter = Interpreter::new();
    match interpreter.interpret_top_level(&stmts) {
        Ok(_) => {
            Ok(interpreter.build_top_level_manifold())
        }
        Err(error) => {
            Err(LangError::Runtime(error))
        }
    }
}
