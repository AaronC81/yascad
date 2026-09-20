use std::collections::HashMap;

use yascad_frontend::InputSourceSpan;

use crate::{EvaluatedParameters, Interpreter, Object, RuntimeError, RuntimeErrorKind};

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

                Object::Absurd => Ok(Object::Absurd),
                
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

fn extract_vector_or_numeric_from_variadic(values: &[Object], span: InputSourceSpan) -> Result<Vec<f64>, RuntimeError> {
    match values.first() {
        Some(Object::Vector(vec)) => {
            if values.len() > 1 {
                return Err(RuntimeError::new(
                    RuntimeErrorKind::IncorrectArity { expected: 1..=1, actual: values.len() },
                    span,
                ));
            }

            vec.into_iter()
                .map(|i| i.as_number(span.clone()))
                .collect::<Result<Vec<_>, _>>()
        },

        Some(Object::Number(_)) => {
            values.iter()
                .map(|i| i.as_number(span.clone()))
                .collect::<Result<Vec<_>, _>>()
        }

        Some(v) => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectType {
                    expected: "vector or numbers".to_owned(),
                    actual: v.describe_type(),
                },
                span,
            ))
        }

        None => {
            return Ok(vec![])
        }
    }
}

fn max_definition() -> FunctionDefinition {
    FunctionDefinition {
        parameters: EvaluatedParameters::variadic("values".to_owned()),
        action: &|_, arguments, span| {
            let values = arguments["values"].as_vector(span.clone())?;
            let numbers = extract_vector_or_numeric_from_variadic(values, span)?;

            Ok(
                numbers.into_iter().reduce(f64::max)
                    .map(Object::Number)
                    .unwrap_or(Object::Null)
            )
        }
    }
}

fn min_definition() -> FunctionDefinition {
    FunctionDefinition {
        parameters: EvaluatedParameters::variadic("values".to_owned()),
        action: &|_, arguments, span| {
            let values = arguments["values"].as_vector(span.clone())?;
            let numbers = extract_vector_or_numeric_from_variadic(values, span)?;

            Ok(
                numbers.into_iter().reduce(f64::min)
                    .map(Object::Number)
                    .unwrap_or(Object::Null)
            )
        }
    }
}

fn sqrt_definition() -> FunctionDefinition {
    FunctionDefinition {
        parameters: EvaluatedParameters::required(vec!["input".to_owned()]),
        action: &|_, arguments, span| {
            let input = arguments["input"].as_number(span.clone())?;
            Ok(Object::Number(input.sqrt()))
        }
    }
}

fn floor_definition() -> FunctionDefinition {
    FunctionDefinition {
        parameters: EvaluatedParameters::required(vec!["input".to_owned()]),
        action: &|_, arguments, span| {
            let input = arguments["input"].as_number(span.clone())?;
            Ok(Object::Number(input.floor()))
        }
    }
}

fn ceil_definition() -> FunctionDefinition {
    FunctionDefinition {
        parameters: EvaluatedParameters::required(vec!["input".to_owned()]),
        action: &|_, arguments, span| {
            let input = arguments["input"].as_number(span.clone())?;
            Ok(Object::Number(input.ceil()))
        }
    }
}

/// Get the implementation for a specific built-in function.
/// 
/// Returns [`None`] if no such operator exists.
pub fn get_builtin_function(name: &str) -> Option<FunctionDefinition> {
    match name {
        "len" => Some(len_definition()),
        "max" => Some(max_definition()),
        "min" => Some(min_definition()),

        "sqrt" => Some(sqrt_definition()),
        "floor" => Some(floor_definition()),
        "ceil" => Some(ceil_definition()),

        _ => None,
    }
}
