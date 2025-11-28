use std::rc::Rc;

use manifold_rs::Manifold;
use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind, manifold_table::ManifoldTableIndex};

#[derive(Debug, Clone)]
pub enum Object {
    Null,
    Number(f64),
    Manifold(ManifoldTableIndex),
    Vector(Vec<Object>),
}

impl Object {
    pub fn describe_type(&self) -> String {
        match self {
            Object::Null => "null",
            Object::Number(_) => "number",
            Object::Manifold(_) => "manifold",
            Object::Vector(_) => "vector",
        }.to_owned()
    }

    pub fn as_number(&self, span: InputSourceSpan) -> Result<f64, RuntimeError> {
        match self {
            Object::Number(num) => Ok(*num),
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectType { expected: "number".to_owned(), actual: self.describe_type() },
                span.clone())
            ),
        }
    }

    pub fn into_manifold(self, span: InputSourceSpan) -> Result<ManifoldTableIndex, RuntimeError> {
        match self {
            Object::Manifold(manifold) => Ok(manifold),
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectType { expected: "manifold".to_owned(), actual: self.describe_type() },
                span.clone())
            ),
        }
    }

    pub fn into_vector(self, span: InputSourceSpan) -> Result<Vec<Object>, RuntimeError> {
        match self {
            Object::Vector(v) => Ok(v),
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectType { expected: "vector".to_owned(), actual: self.describe_type() },
                span.clone())
            ),
        }
    }
}
