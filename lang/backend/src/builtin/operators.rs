use manifold_rs::Manifold;
use yascad_frontend::InputSourceSpan;

use crate::{Interpreter, RuntimeError, RuntimeErrorKind, builtin::{accept_arguments, accept_vec2_argument, accept_vec3_argument, reject_arguments}, geometry_table::{GeometryDisposition, GeometryTableEntry, GeometryTableIndex}, object::Object};

pub type OperatorDefinition = &'static dyn Fn(&mut Interpreter, Vec<Object>, Vec<GeometryTableIndex>, InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError>;

fn translate(interpreter: &mut Interpreter, arguments: Vec<Object>, children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
    match interpreter.manifold_table.remove_many_into_union(children, span.clone())? {
        (GeometryTableEntry::Manifold(manifold), d) => {
            let (x, y, z) = accept_vec3_argument(arguments, span.clone())?;
            Ok((GeometryTableEntry::Manifold(manifold.translate(x, y, z)), d))
        },

        (GeometryTableEntry::CrossSection(cross_section), d) => {
            let (x, y) = accept_vec2_argument(arguments, span.clone())?;
            Ok((GeometryTableEntry::CrossSection(cross_section.translate(x, y)), d))
        },
    }
}

fn union(interpreter: &mut Interpreter, arguments: Vec<Object>, children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
    reject_arguments(arguments, &span)?;
    interpreter.manifold_table.remove_many_into_union(children, span)
}

fn difference(interpreter: &mut Interpreter, arguments: Vec<Object>, mut children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
    reject_arguments(arguments, &span)?;

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

fn linear_extrude(interpreter: &mut Interpreter, arguments: Vec<Object>, children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
    let [height] = accept_arguments(arguments, &span)?;
    let height = height.as_number(span.clone())?;

    let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;
    let GeometryTableEntry::CrossSection(cross_section) = geom
    else { return Err(RuntimeError::new(RuntimeErrorKind::Requires2DGeometry, span.clone())) };

    Ok((GeometryTableEntry::Manifold(Manifold::extrude(cross_section.polygons(), height)), disp))
}

fn buffer(interpreter: &mut Interpreter, arguments: Vec<Object>, children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
    reject_arguments(arguments, &span)?;

    let (geom, _) = interpreter.manifold_table.remove_many_into_union(children, span)?;
    Ok((geom, GeometryDisposition::Virtual))
}

/// Get the implementation for a specific built-in operator.
/// 
/// Returns [`None`] if no such operator exists.
pub fn get_builtin_operator(name: &str) -> Option<OperatorDefinition> {
    match name {
        "translate" => Some(&translate),
        "union" => Some(&union),
        "difference" => Some(&difference),
        "linear_extrude" => Some(&linear_extrude),
        "buffer" => Some(&buffer),

        _ => None,
    }
}
