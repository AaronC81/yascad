use std::rc::Rc;

use manifold_rs::Manifold;
use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind, manifold_table::ManifoldTableIndex};

#[derive(Debug, Clone)]
pub enum Object {
    Null,
    Number(f64),
    Manifold(ManifoldTableIndex),
}

impl Object {
    pub fn describe_type(&self) -> String {
        match self {
            Object::Null => "null",
            Object::Number(_) => "number",
            Object::Manifold(_) => "manifold",
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
}
