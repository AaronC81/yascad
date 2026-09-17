use std::collections::HashMap;

use yascad_frontend::InputSourceSpan;

use crate::{EvaluatedParameters, Interpreter, Object, RuntimeError};

/// Defines the parameters and behaviour of a built-in module.
/// 
/// The `action` can assume that all of its arguments have been validated - all of the keys defined
/// in `parameters` definitely exist.
#[derive(Clone)]
pub struct FunctionDefinition {
    pub parameters: EvaluatedParameters,
    pub action: &'static dyn Fn(&mut Interpreter, HashMap<String, Object>, InputSourceSpan) -> Result<Object, RuntimeError>,
}

fn len_definition() -> FunctionDefinition {
    FunctionDefinition {
        parameters: EvaluatedParameters::required(vec!["subject".to_owned()]),
        action: &|_, arguments, span| {
            let subject = &arguments["subject"];
            match subject {
                Object::String(str) => Ok(Object::Number(str.len() as f64)),
                Object::Vector(vec) => Ok(Object::Number(vec.len() as f64)),
                
                Object::Null
                | Object::Number(_)
                | Object::Boolean(_)
                | Object::Manifold(_)
                | Object::CrossSection(_)
                | Object::EmptyGeometry => {
                    Err(RuntimeError::new(
                        crate::RuntimeErrorKind::IncorrectType {
                            expected: "string or vector".to_owned(),
                            actual: subject.describe_type(),
                        },
                        span,
                    ))
                }
            }
        },
    }
}

/// Get the implementation for a specific built-in function.
/// 
/// Returns [`None`] if no such operator exists.
pub fn get_builtin_function(name: &str) -> Option<FunctionDefinition> {
    match name {
        "len" => Some(len_definition()),

        _ => None,
    }
}
