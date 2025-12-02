use manifold_rs::Vec3;
use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind, geometry_table::{GeometryTable, GeometryTableIndex}};

#[derive(Debug, Clone)]
pub enum Object {
    Null,
    Number(f64),
    Manifold(GeometryTableIndex),
    CrossSection(GeometryTableIndex),
    Vector(Vec<Object>),
}

impl Object {
    pub fn describe_type(&self) -> String {
        match self {
            Object::Null => "null",
            Object::Number(_) => "number",
            Object::Manifold(_) => "3D manifold",
            Object::CrossSection(_) => "2D cross-section",
            Object::Vector(_) => "vector",
        }.to_owned()
    }

    #[allow(clippy::get_first)] // `get(1/2)` mixed with `first()` is confusing
    pub fn get_field(&self, field: &str, manifold_table: &GeometryTable) -> Option<Object> {
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
                let bounding_box = manifold_table.get(index).unwrap_manifold().bounding_box();

                match field {
                    "origin" | "min_point" => Some(bounding_box.min_point().into()),
                    "max_point" => Some(bounding_box.max_point().into()),
                    "size" => Some(bounding_box.size().into()),
                    _ => None,
                }
            },

            // TODO: fields on cross-sections
            Object::CrossSection(_) => todo!(),
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

    pub fn into_manifold(self, span: InputSourceSpan) -> Result<GeometryTableIndex, RuntimeError> {
        match self {
            Object::Manifold(manifold) => Ok(manifold),
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectType { expected: "3D manifold".to_owned(), actual: self.describe_type() },
                span.clone())
            ),
        }
    }

    pub fn into_cross_section(self, span: InputSourceSpan) -> Result<GeometryTableIndex, RuntimeError> {
        match self {
            Object::CrossSection(xs) => Ok(xs),
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectType { expected: "2D cross-section".to_owned(), actual: self.describe_type() },
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
    
    /// Assume that the object is a vector with two number components, then unpacks the components.
    /// 
    /// If it doesn't match this form, returns an appropriate [`RuntimeError`].
    pub fn into_2d_vector(self, span: InputSourceSpan) -> Result<(f64, f64), RuntimeError> {
        let vector = self.into_vector(span.clone())?;
        if vector.len() != 2 {
            return Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectVectorLength { expected: 2..=2, actual: vector.len() },
                span,
            ));
        }

        let x = vector[0].as_number(span.clone())?;
        let y = vector[1].as_number(span.clone())?;

        Ok((x, y))
    }

    /// Assume that the object is a vector with either two or three number components, then unpacks
    /// the components. If the third component is omitted, it defaults to 0.
    /// 
    /// If it doesn't match this form, returns an appropriate [`RuntimeError`].
    pub fn into_3d_vector(self, span: InputSourceSpan) -> Result<(f64, f64, f64), RuntimeError> {
        let vector = self.into_vector(span.clone())?;
        if vector.len() != 2 && vector.len() != 3 {
            return Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectVectorLength { expected: 2..=3, actual: vector.len() },
                span,
            ));
        }

        let x = vector[0].as_number(span.clone())?;
        let y = vector[1].as_number(span.clone())?;

        let z =
            if vector.len() == 3 {
                vector[2].as_number(span.clone())?
            } else {
                0.0
            };

        Ok((x, y, z))
    }
}

impl From<Vec3<f64>> for Object {
    fn from(value: Vec3<f64>) -> Self {
        Self::Vector(vec![Self::Number(value.x), Self::Number(value.y), Self::Number(value.z)])
    }
}
