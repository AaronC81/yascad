use std::collections::HashMap;

use manifold_rs::Manifold;
use yascad_frontend::{InputSourceSpan, Parameters};

use crate::{Interpreter, RuntimeError, RuntimeErrorKind, geometry_table::{GeometryDisposition, GeometryTableEntry, GeometryTableIndex}, object::Object};

/// Defines the parameters and behaviour of a built-in operator.
/// 
/// The `action` can assume that all of its arguments have been validated - all of the keys defined
/// in `parameters` definitely exist.
#[derive(Clone)]
pub struct OperatorDefinition {
    pub parameters: Parameters,
    pub action: &'static dyn Fn(&mut Interpreter, HashMap<String, Object>, Vec<GeometryTableIndex>, InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError>,
}

fn translate_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: Parameters::required(vec!["v".to_owned()]),
        action: &|interpreter, arguments, children, span| {
            match interpreter.manifold_table.remove_many_into_union(children, span.clone())? {
                (GeometryTableEntry::Manifold(manifold), d) => {
                    let (x, y, z) = arguments["v"].as_3d_vector(span)?;
                    Ok((GeometryTableEntry::Manifold(manifold.translate(x, y, z)), d))
                },

                (GeometryTableEntry::CrossSection(cross_section), d) => {
                    let (x, y) = arguments["v"].as_2d_vector(span)?;
                    Ok((GeometryTableEntry::CrossSection(cross_section.translate(x, y)), d))
                },
            }
        }
    }
}

fn union_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: Parameters::empty(),
        action: &|interpreter, _, children, span| {
            interpreter.manifold_table.remove_many_into_union(children, span)
        }
    }
}

fn difference_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: Parameters::empty(),
        action: &|interpreter, _, mut children, span| {
            if children.is_empty() {
                return Err(RuntimeError::new(RuntimeErrorKind::ChildrenExpected, span))
            }

            let (minuend, disp) = interpreter.manifold_table.remove(children.remove(0));
            if children.is_empty() {
                return Ok((minuend, disp))
            }
            
            let (subtrahend, _) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;
            match (minuend, subtrahend) {
                (GeometryTableEntry::Manifold(minuend_manifold), GeometryTableEntry::Manifold(subtrahend_manifold)) => {
                    Ok((GeometryTableEntry::Manifold(minuend_manifold.difference(&subtrahend_manifold)), disp))
                },

                (GeometryTableEntry::CrossSection(minuend_cross_section), GeometryTableEntry::CrossSection(subtrahend_cross_section)) => {
                    Ok((GeometryTableEntry::CrossSection(minuend_cross_section.difference(&subtrahend_cross_section)), disp))
                },

                _ => {
                    Err(RuntimeError::new(RuntimeErrorKind::MixedGeometryDimensions, span))
                }
            }
        }
    }
}

fn linear_extrude_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: Parameters::required(vec!["h".to_owned()]),
        action: &|interpreter, arguments, children, span| {
            let height = arguments["h"].as_number(span.clone())?;

            let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;
            let GeometryTableEntry::CrossSection(cross_section) = geom
            else { return Err(RuntimeError::new(RuntimeErrorKind::Requires2DGeometry, span.clone())) };

            Ok((GeometryTableEntry::Manifold(Manifold::extrude(cross_section.polygons(), height)), disp))
        },
    }
}

fn buffer_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: Parameters::empty(),
        action: &|interpreter, _, children, span| {
            let (geom, _) = interpreter.manifold_table.remove_many_into_union(children, span)?;
            Ok((geom, GeometryDisposition::Virtual))
        },
    }
}

/// Get the implementation for a specific built-in operator.
/// 
/// Returns [`None`] if no such operator exists.
pub fn get_builtin_operator(name: &str) -> Option<OperatorDefinition> {
    match name {
        "translate" => Some(translate_definition()),
        "union" => Some(union_definition()),
        "difference" => Some(difference_definition()),
        "linear_extrude" => Some(linear_extrude_definition()),
        "buffer" => Some(buffer_definition()),

        _ => None,
    }
}
