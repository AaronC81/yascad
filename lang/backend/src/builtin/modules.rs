use std::{collections::HashMap, fs::read_to_string};

use manifold_csg::{CrossSection, FillRule, Manifold};
use manifold_csg_ext::{svg_to_cross_section, text_to_cross_section};
use yascad_frontend::InputSourceSpan;

use crate::{EvaluatedParameters, Interpreter, RuntimeError, RuntimeErrorKind, geometry_table::{GeometryDisposition, GeometryTableIndex}, object::Object};

/// Defines the parameters and behaviour of a built-in module.
/// 
/// The `action` can assume that all of its arguments have been validated - all of the keys defined
/// in `parameters` definitely exist.
#[derive(Clone)]
pub struct ModuleDefinition {
    pub parameters: EvaluatedParameters,
    pub action: &'static dyn Fn(&mut Interpreter, HashMap<String, Object>, Option<&[GeometryTableIndex]>, InputSourceSpan) -> Result<Object, RuntimeError>,
}

fn cube_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters::required(vec!["size".to_owned()]),
        action: &|interpreter, arguments, _, span| {
            let (x, y, z) = match &arguments["size"] {
                Object::Vector(_) => arguments["size"].as_3d_vector(span)?,
                Object::Number(n) => (*n, *n, *n),
                o => {
                    return Err(RuntimeError::new(RuntimeErrorKind::IncorrectType {
                        expected: "number or vector".to_owned(),
                        actual: o.describe_type()
                    }, span))
                }
            };
            Ok(Object::Manifold(interpreter.manifold_table.add_manifold(Manifold::cube(x, y, z, false), GeometryDisposition::Physical)))
        },
    }
}

fn cylinder_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters {
            required: vec!["h".to_owned()],
            optional: vec![("r".to_owned(), Object::Null)],
            optional_named_only: vec![("d".to_owned(), Object::Null)],
            variadic: None,
        },
        action: &|interpreter, arguments, _, span| {
            // TODO: needs to support cone forms
            let height = arguments["h"].as_number(span.clone())?;
            let radius = radius_argument(&arguments, span)?;

            Ok(Object::Manifold(interpreter.manifold_table.add_manifold(Manifold::cylinder(height, radius, radius, interpreter.circle_segments, false), GeometryDisposition::Physical)))
        },
    }
}

fn sphere_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters {
            required: vec![],
            optional: vec![("r".to_owned(), Object::Null)],
            optional_named_only: vec![("d".to_owned(), Object::Null)],
            variadic: None,
        },
        action: &|interpreter, arguments, _, span| {
            let radius = radius_argument(&arguments, span)?;

            Ok(Object::Manifold(interpreter.manifold_table.add_manifold(Manifold::sphere(radius, interpreter.circle_segments), GeometryDisposition::Physical)))
        },
    }
}

fn square_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters::required(vec!["size".to_owned()]),
        action: &|interpreter, arguments: HashMap<String, Object>, _, span| {
            let (x, y) = match &arguments["size"] {
                Object::Vector(_) => arguments["size"].as_2d_vector(span)?,
                Object::Number(n) => (*n, *n),
                o => {
                    return Err(RuntimeError::new(RuntimeErrorKind::IncorrectType {
                        expected: "number or vector".to_owned(),
                        actual: o.describe_type()
                    }, span))
                }
            };
            Ok(Object::CrossSection(interpreter.manifold_table.add_cross_section(CrossSection::square(x, y, false), GeometryDisposition::Physical)))
        }
    }
}

fn circle_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters {
            required: vec![],
            optional: vec![("r".to_owned(), Object::Null)],
            optional_named_only: vec![("d".to_owned(), Object::Null)],
            variadic: None,
        },
        action: &|interpreter, arguments: HashMap<String, Object>, _, span| {
            let radius = radius_argument(&arguments, span)?;
            Ok(Object::CrossSection(interpreter.manifold_table.add_cross_section(CrossSection::circle(radius, interpreter.circle_segments), GeometryDisposition::Physical)))
        }
    }
}

fn copy_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters::required(vec!["source".to_owned()]),
        action: &|interpreter, arguments, _, span| {
            // Note: Even if geometry is being copied in a virtual disposition, we can make it physical here.
            // The `buffer` will "downgrade" it later.
            match &arguments["source"] {
                Object::Manifold(geometry_table_index) => {
                    let manifold = interpreter.manifold_table.get(&geometry_table_index);
                    let copied_manifold = interpreter.manifold_table.add(manifold.clone(), GeometryDisposition::Physical);
                    Ok(Object::Manifold(copied_manifold))
                }
                Object::CrossSection(geometry_table_index) => {
                    let cross_section = interpreter.manifold_table.get(&geometry_table_index);
                    let copied_cross_section = interpreter.manifold_table.add(cross_section.clone(), GeometryDisposition::Physical);
                    Ok(Object::CrossSection(copied_cross_section))
                }
                Object::EmptyGeometry => Ok(Object::EmptyGeometry),

                obj => {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::IncorrectType { expected: "2D or 3D geometry".to_owned(), actual: obj.describe_type() },
                        span
                    ));
                }
            }
        },
    }
}

fn polygon_definition() -> ModuleDefinition {
    // TODO: does not support `paths` or `convexity`
    ModuleDefinition {
        parameters: EvaluatedParameters::required(vec!["points".to_owned()]),
        action: &|interpreter, arguments, _, span| {
            let points = arguments["points"].as_vector(span.clone())?
                .iter()
                .map(|pt| pt.as_2d_vector(span.clone()))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|(x, y)| [x, y])
                .collect::<Vec<_>>();

            Ok(Object::CrossSection(interpreter.manifold_table.add_cross_section(
                // OpenSCAD uses even-odd apparently
                // https://github.com/elalish/manifold/issues/1707#issuecomment-4939305450
                CrossSection::from_simple_polygon_with_fill_rule(&points, FillRule::EvenOdd),
                GeometryDisposition::Physical,
            )))
        },
    }
}

fn text_definition() -> ModuleDefinition {
    // TODO: support ANY parameters
    ModuleDefinition {
        parameters: EvaluatedParameters::required(vec!["text".to_owned()]),
        action: &|interpreter, arguments, _, span| {
            Ok(Object::CrossSection(interpreter.manifold_table.add_cross_section(
                text_to_cross_section(&arguments["text"].as_string(span)?),
                GeometryDisposition::Physical,
            )))
        },
    }
}

pub(crate) fn get_children(
    interpreter: &mut Interpreter,
    arguments: HashMap<String, Object>,
    operator_children: Option<&[GeometryTableIndex]>,
    span: InputSourceSpan,
) -> Result<Vec<GeometryTableIndex>, RuntimeError> {
    let Some(children) = operator_children
    else {
        return Err(RuntimeError::new(RuntimeErrorKind::ChildrenInvalid, span));
    };

    let selected_children = match &arguments["index"] {
        Object::Null => children.to_vec(),

        obj@Object::Number(_) => {
            let index = obj.as_index(span.clone())?;

            match children.get(index) {
                Some(child) => vec![child.clone()],
                None => return Err(RuntimeError::new(
                    RuntimeErrorKind::ChildOutOfRange(index),
                    span,
                )),
            }

        },

        Object::Vector(vec) => {
            let indices = vec.into_iter()
                .map(|o| o.as_index(span.clone()))
                .collect::<Result<Vec<_>, _>>()?;

            for index in &indices {
                if *index >= children.len() {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::ChildOutOfRange(*index),
                        span,
                    ));
                }
            }

            indices
                .into_iter()
                .map(|i| children[i].clone())
                .collect()
        },

        o => return Err(RuntimeError::new(
            RuntimeErrorKind::IncorrectType { expected: "number or vector".to_owned(), actual: o.describe_type() },
            span,
        )),
    };

    // The children are temporary virtual manifolds.
    // Copy them as physical and then build a union of all of the copies.
    Ok(selected_children.iter()
        .map(|child| {
            let m = interpreter.manifold_table.get(child).clone();
            interpreter.manifold_table.add(m, GeometryDisposition::Physical)
        })
        .collect::<Vec<_>>())
}

pub(crate) fn children_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters::new(vec![], vec![("index".to_owned(), Object::Null)]),
        action: &|interpreter, arguments, operator_children, span| {
            let copied_children = get_children(interpreter, arguments, operator_children, span.clone())?;

            let (geom, disp) = interpreter.manifold_table.remove_many_into_union(copied_children, span)?;
            Ok(interpreter.manifold_table.add_into_object(geom, disp))
        }
    }
}

fn __debug_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters::required(vec!["o".to_owned()]),
        action: &|interpreter, arguments, _, _| {
            (interpreter.debug_hook)(&arguments["o"]);
            Ok(Object::Null)
        },
    }
}

// TODO: all horribly temporary
fn __svg_definition() -> ModuleDefinition {
    ModuleDefinition {
        parameters: EvaluatedParameters::required(vec!["path".to_owned()]),
        action: &|interpreter, arguments, _, span| {
            let path = &arguments["path"].as_string(span)?;
            let file_contents = read_to_string(path).unwrap();

            let cross_section = svg_to_cross_section(&file_contents, 0.25).unwrap();
            Ok(Object::CrossSection(interpreter.manifold_table.add_cross_section(cross_section, GeometryDisposition::Physical)))
        },
    }
}

/// Given an argument map which may contain a non-null `r` or `d`, gets the radius.
pub fn radius_argument(arguments: &HashMap<String, Object>, span: InputSourceSpan) -> Result<f64, RuntimeError> {
    match (&arguments["r"], &arguments["d"]) {
        (Object::Null, Object::Null) =>
            Err(RuntimeError::new(RuntimeErrorKind::AssertionError(
                "neither \"r\" nor \"d\" argument is given, but one must be specified".to_owned()
            ), span)),
        
        (radius, Object::Null) => radius.as_number(span),
        (Object::Null, diameter) => diameter.as_number(span).map(|n| n / 2.0),

        (_, _) =>
            Err(RuntimeError::new(RuntimeErrorKind::AssertionError(
                "both \"r\" and \"d\" arguments are given, but only one must be specified".to_owned()
            ), span)),
    }
}

/// Get the implementation for a specific built-in module.
/// 
/// Returns [`None`] if no such module exists.
pub fn get_builtin_module(name: &str) -> Option<ModuleDefinition> {
    match name {
        "cube" => Some(cube_definition()),
        "cylinder" => Some(cylinder_definition()),
        "sphere" => Some(sphere_definition()),

        "square" => Some(square_definition()),
        "circle" => Some(circle_definition()),
        "polygon" => Some(polygon_definition()),
        "text" => Some(text_definition()),

        "copy" => Some(copy_definition()),
        "children" => Some(children_definition()),

        "__debug" => Some(__debug_definition()),
        "__svg" => Some(__svg_definition()),

        _ => None,
    }
}
