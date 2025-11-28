use std::rc::Rc;

use manifold_rs::{Manifold, Vec3};
use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind, manifold_table::{ManifoldTable, ManifoldTableIndex}};

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

    pub fn get_field(&self, field: &str, manifold_table: &ManifoldTable) -> Option<Object> {
        match self {
            Object::Null | Object::Number(_) => None,

            Object::Vector(objects) => {
                match field {
                    "x" => Some(objects.get(0).cloned().unwrap_or(Object::Null)),
                    "y" => Some(objects.get(1).cloned().unwrap_or(Object::Null)),
                    "z" => Some(objects.get(2).cloned().unwrap_or(Object::Null)),
                    _ => None,
                }
            },

            Object::Manifold(index) => {
                let bounding_box = manifold_table.get(index).bounding_box();

                match field {
                    "origin" | "min_point" => Some(bounding_box.min_point().into()),
                    "max_point" => Some(bounding_box.max_point().into()),
                    "size" => Some(bounding_box.size().into()),
                    _ => None,
                }
            }
        }
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

impl From<Vec3> for Object {
    fn from(value: Vec3) -> Self {
        Self::Vector(vec![Self::Number(value.x), Self::Number(value.y), Self::Number(value.z)])
    }
}
