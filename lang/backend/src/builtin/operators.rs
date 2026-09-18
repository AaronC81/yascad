use std::collections::HashMap;

use manifold_csg::{CrossSection, Manifold};
use yascad_frontend::InputSourceSpan;

use crate::{EvaluatedParameters, Interpreter, RuntimeError, RuntimeErrorKind, geometry_table::{GeometryDisposition, GeometryTableEntry, GeometryTableIndex}, object::Object};

/// Defines the parameters and behaviour of a built-in operator.
/// 
/// The `action` can assume that all of its arguments have been validated - all of the keys defined
/// in `parameters` definitely exist.
#[derive(Clone)]
pub struct OperatorDefinition {
    pub parameters: EvaluatedParameters,
    pub action: &'static dyn Fn(&mut Interpreter, HashMap<String, Object>, Vec<GeometryTableIndex>, InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError>,
}

fn translate_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::required(vec!["v".to_owned()]),
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

                empty@(GeometryTableEntry::EmptyGeometry, _) => Ok(empty),
            }
        }
    }
}

fn union_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::empty(),
        action: &|interpreter, _, children, span| {
            interpreter.manifold_table.remove_many_into_union(children, span)
        }
    }
}

fn difference_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::empty(),
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

fn intersection_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::empty(),
        action: &|interpreter, _, mut children, span| {
            if children.is_empty() {
                return Ok(interpreter.manifold_table.get_empty())
            }

            let (mut result, disp) = interpreter.manifold_table.remove(children.remove(0));
            
            loop {
                if children.is_empty() {
                    return Ok((result, disp))
                }

                let (this_child, _) = interpreter.manifold_table.remove(children.remove(0));
                result = match (result, this_child) {
                    (GeometryTableEntry::Manifold(result_manifold), GeometryTableEntry::Manifold(this_manifold)) => {
                        GeometryTableEntry::Manifold(result_manifold.intersection(&this_manifold))
                    },

                    (GeometryTableEntry::CrossSection(result_cross_section), GeometryTableEntry::CrossSection(this_cross_section)) => {
                        GeometryTableEntry::CrossSection(result_cross_section.intersection(&this_cross_section))
                    },

                    _ => {
                        return Err(RuntimeError::new(RuntimeErrorKind::MixedGeometryDimensions, span))
                    }
                }
            }
        }
    }
}

fn hull_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::empty(),
        action: &|interpreter, _, children, span| {
            let (entry, disp) = interpreter.manifold_table.remove_many_into_union(children, span)?;
            match entry {
                GeometryTableEntry::Manifold(manifold) => {
                    Ok((GeometryTableEntry::Manifold(manifold.hull()), disp))
                },
                GeometryTableEntry::CrossSection(cross_section) => {
                    Ok((GeometryTableEntry::CrossSection(cross_section.hull()), disp))
                },
                GeometryTableEntry::EmptyGeometry => {
                    Ok((GeometryTableEntry::EmptyGeometry, disp))
                }
            }
        }
    }
}

fn linear_extrude_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::required(vec!["h".to_owned()]),
        action: &|interpreter, arguments, children, span| {
            let height = arguments["h"].as_number(span.clone())?;

            let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;
            let GeometryTableEntry::CrossSection(cross_section) = geom
            else { return Err(RuntimeError::new(RuntimeErrorKind::Requires2DGeometry, span.clone())) };

            Ok((GeometryTableEntry::Manifold(Manifold::extrude(&cross_section, height)), disp))
        },
    }
}

fn rotate_extrude_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::new(
            vec![],
            vec![("angle".to_owned(), Object::Number(360.0))],
        ),
        action: &|interpreter, arguments, children, span| {
            let angle = arguments["angle"].as_number(span.clone())?;

            let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;
            let GeometryTableEntry::CrossSection(cross_section) = geom
            else { return Err(RuntimeError::new(RuntimeErrorKind::Requires2DGeometry, span.clone())) };

            Ok((GeometryTableEntry::Manifold(Manifold::revolve(&cross_section, interpreter.circle_segments, angle)), disp))
        },
    }
}

fn rotate_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::required(vec!["v".to_owned()]),
        action: &|interpreter, arguments, children, span| {
            let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;

            let arg = &arguments["v"];
            let (x, y, z) = match &arg {
                Object::Number(angle) => (0.0, 0.0, *angle),
                Object::Vector(_) => arg.as_3d_vector(span.clone())?,
                _ => return Err(RuntimeError::new(
                    RuntimeErrorKind::IncorrectType {
                        expected: "number or 3D vector".to_owned(),
                        actual: arg.describe_type(),
                    },
                    span
                ))
            };

            Ok((match geom {
                GeometryTableEntry::Manifold(manifold) => {
                    GeometryTableEntry::Manifold(manifold.rotate(x, y, z))
                }
                GeometryTableEntry::CrossSection(cross_section) => {
                    if x != 0.0 || y != 0.0 {
                        return Err(RuntimeError::new(RuntimeErrorKind::CannotRotate2DGeometryOnAxis, span));
                    }
                    GeometryTableEntry::CrossSection(cross_section.rotate(z))
                }
                GeometryTableEntry::EmptyGeometry => geom,
            }, disp))
        },
    }
}

fn scale_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::required(vec!["v".to_owned()]),
        action: &|interpreter, arguments, children, span| {
            let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;

            Ok((match geom {
                GeometryTableEntry::Manifold(manifold) => {
                    let (x, y, z) = arguments["v"].as_3d_vector(span.clone())?;
                    GeometryTableEntry::Manifold(manifold.scale(x, y, z))
                }
                GeometryTableEntry::CrossSection(cross_section) => {
                    let (x, y) = arguments["v"].as_2d_vector(span.clone())?;
                    GeometryTableEntry::CrossSection(cross_section.scale(x, y))
                }
                GeometryTableEntry::EmptyGeometry => geom,
            }, disp))
        },
    }
}

fn mirror_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::required(vec!["v".to_owned()]),
        action: &|interpreter, arguments, children, span| {
            let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;

            Ok((match geom {
                GeometryTableEntry::Manifold(manifold) => {
                    let (x, y, z) = arguments["v"].as_3d_vector(span.clone())?;
                    GeometryTableEntry::Manifold(manifold.mirror([x, y, z]))
                }
                GeometryTableEntry::CrossSection(cross_section) => {
                    let (x, y) = arguments["v"].as_2d_vector(span.clone())?;
                    GeometryTableEntry::CrossSection(cross_section.mirror(x, y))
                }
                GeometryTableEntry::EmptyGeometry => geom,
            }, disp))
        },
    }
}

fn buffer_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::empty(),
        action: &|interpreter, _, children, span| {
            let (geom, _) = interpreter.manifold_table.remove_many_into_union(children, span)?;
            Ok((geom, GeometryDisposition::Virtual))
        },
    }
}

fn output_definition() -> OperatorDefinition {
    OperatorDefinition {
        parameters: EvaluatedParameters::required(vec!["name".to_owned()]),
        action: &|interpreter, arguments, children, span| {
            let name = arguments["name"].as_string(span.clone())?;
            let (geom, _) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;

            interpreter.add_output(&name, geom.clone(), span)?;

            Ok((geom, GeometryDisposition::Physical))
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
        "intersection" => Some(intersection_definition()),
        "hull" => Some(hull_definition()),
        "linear_extrude" => Some(linear_extrude_definition()),
        "rotate_extrude" => Some(rotate_extrude_definition()),
        "rotate" => Some(rotate_definition()),
        "scale" => Some(scale_definition()),
        "mirror" => Some(mirror_definition()),
        "buffer" => Some(buffer_definition()),
        "output" => Some(output_definition()),

        _ => None,
    }
}
