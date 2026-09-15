use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind, geometry_table::{GeometryTable, GeometryTableIndex}};

#[derive(Debug, Clone)]
pub enum Object {
    Null,
    Number(f64),
    String(String),
    Boolean(bool),
    Vector(Vec<Object>),

    Manifold(GeometryTableIndex),
    CrossSection(GeometryTableIndex),
    EmptyGeometry,
}

impl Object {
    pub fn describe_type(&self) -> String {
        match self {
            Object::Null => "null",
            Object::Number(_) => "number",
            Object::String(_) => "string",
            Object::Boolean(_) => "boolean",
            Object::Vector(_) => "vector",

            Object::Manifold(_) => "3D manifold",
            Object::CrossSection(_) => "2D cross-section",
            Object::EmptyGeometry => "empty geometry",
        }.to_owned()
    }

    #[allow(clippy::get_first)] // `get(1/2)` mixed with `first()` is confusing
    pub fn get_field(&self, field: &str, manifold_table: &GeometryTable) -> Option<Object> {
        match self {
            Object::Null | Object::Number(_) | Object::String(_) | Object::Boolean(_) => None,

            Object::Vector(objects) => {
                match field {
                    "x" => Some(objects.get(0).cloned().unwrap_or(Object::Null)),
                    "y" => Some(objects.get(1).cloned().unwrap_or(Object::Null)),
                    "z" => Some(objects.get(2).cloned().unwrap_or(Object::Null)),
                    "length" => Some(Object::Number(objects.len() as f64)),
                    _ => None,
                }
            },

            Object::Manifold(index) => {
                let bounding_box = manifold_table.get(index)
                    .unwrap_manifold()
                    .bounding_box();

                if let Some(bounding_box) = bounding_box {
                    match field {
                        "origin" | "min_point" => Some(bounding_box.min().into()),
                        "max_point" => Some(bounding_box.max().into()),
                        "size" => Some(bounding_box.dimensions().into()),
                        _ => None,
                    }
                } else {
                    self.get_field_of_empty_geometry(field)
                }
            },

            Object::CrossSection(index) => {
                let bounding_rect = manifold_table.get(index).unwrap_cross_section().bounds_rect2();

                match field {
                    "origin" | "min_point" => Some([bounding_rect.min_x, bounding_rect.min_y].into()),
                    "max_point" => Some([bounding_rect.max_x, bounding_rect.max_y].into()),
                    "size" => Some(
                        [
                            bounding_rect.max_x - bounding_rect.min_x,
                            bounding_rect.max_y - bounding_rect.min_y,
                        ].into()
                    ),
                    _ => None,
                }
            },

            Object::EmptyGeometry => {
                self.get_field_of_empty_geometry(field)
            }
        }
    }

    fn get_field_of_empty_geometry(&self, field: &str) -> Option<Object> {
        let empty_vector = Object::Vector(vec![
            Object::Number(0.0),
            Object::Number(0.0),
            Object::Number(0.0),
        ]);

        match field {
            "origin" | "min_point" | "max_point" | "size" => Some(empty_vector),
            _ => None,
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

    pub fn as_string(&self, span: InputSourceSpan) -> Result<String, RuntimeError> {
        match self {
            Object::String(str) => Ok(str.clone()),
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectType { expected: "string".to_owned(), actual: self.describe_type() },
                span.clone())
            ),
        }
    }

    pub fn as_boolean(&self, span: InputSourceSpan) -> Result<bool, RuntimeError> {
        match self {
            Object::Boolean(bool) => Ok(*bool),
            _ => Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectType { expected: "boolean".to_owned(), actual: self.describe_type() },
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

    pub fn as_vector(&self, span: InputSourceSpan) -> Result<&[Object], RuntimeError> {
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
    pub fn as_2d_vector(&self, span: InputSourceSpan) -> Result<(f64, f64), RuntimeError> {
        let vector = self.as_vector(span.clone())?;
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
    pub fn as_3d_vector(&self, span: InputSourceSpan) -> Result<(f64, f64, f64), RuntimeError> {
        let vector = self.as_vector(span.clone())?;
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

impl From<[f64; 3]> for Object {
    fn from([x, y, z]: [f64; 3]) -> Self {
        Self::Vector(vec![Self::Number(x), Self::Number(y), Self::Number(z)])
    }
}

impl From<[f64; 2]> for Object {
    fn from([x, y]: [f64; 2]) -> Self {
        Self::Vector(vec![Self::Number(x), Self::Number(y)])
    }
}

// Not full equivalence because manifolds/cross-sections are never equal
impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(l), Self::Number(r)) => l == r,
            (Self::String(l), Self::String(r)) => l == r,
            (Self::Boolean(l), Self::Boolean(r)) => l == r,
            (Self::Vector(l), Self::Vector(r)) => l == r,
            (Self::Null, Self::Null) => true,

            // Because it's a footgun, geometry never compares
            (Self::Manifold(_), Self::Manifold(_)) => false,
            (Self::CrossSection(_), Self::CrossSection(_)) => false,
            (Self::EmptyGeometry, Self::EmptyGeometry) => false,

            // Not using `_` so we get exhaustiveness error for new variants
            (Self::Number(_), _)
            | (Self::String(_), _)
            | (Self::Boolean(_), _)
            | (Self::Vector(_), _)
            | (Self::Null, _)
            | (Self::Manifold(_), _)
            | (Self::CrossSection(_), _)
            | (Self::EmptyGeometry, _)
                => false,
        }
    }
}
