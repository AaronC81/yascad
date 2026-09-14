use std::{collections::HashMap, rc::Rc};

use manifold_csg::Manifold;

use yascad_backend::{Interpreter, Object};
use yascad_frontend::{Parser, tokenize};
pub use yascad_frontend::{InputSource, InputSourceOrigin, ParseError, TokenizeError};
pub use yascad_backend::RuntimeError;

#[derive(Debug, Clone)]
pub enum LangError {
    Tokenize(Vec<TokenizeError>),
    Parser(Vec<ParseError>),
    Runtime(RuntimeError),
}

#[derive(Default)]
pub struct BuildModelOptions {
    pub debug_hook: Option<Box<dyn Fn(&Object) + 'static>>,
}

#[derive(Debug, Clone)]
pub struct BuildModelOutput {
    pub main: Manifold,
    pub outputs: HashMap<String, Manifold>,
}

pub fn build_model(source: InputSource, options: BuildModelOptions) -> Result<BuildModelOutput, LangError> {
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

    if let Some(debug_hook) = options.debug_hook {
        interpreter.set_debug_hook(debug_hook);
    }

    match interpreter.interpret_top_level(&stmts) {
        Ok(_) => {
            let main = interpreter.build_top_level_manifold();

            let outputs = interpreter.outputs
                .iter()
                .map(|(k, v)| (k.clone(), v.as_manifold_for_display()))
                .collect::<HashMap<_, _>>();

            Ok(BuildModelOutput { main, outputs })
        }
        Err(error) => {
            Err(LangError::Runtime(error))
        }
    }
}
