use manifold_rs::Manifold;
use yascad_frontend::InputSourceSpan;

use crate::{Interpreter, RuntimeError, RuntimeErrorKind, builtin::{accept_arguments, accept_vec2_argument, accept_vec3_argument, reject_arguments}, geometry_table::{GeometryDisposition, GeometryTableEntry, GeometryTableIndex}, object::Object};

pub type OperatorDefinition = &'static dyn Fn(&mut Interpreter, Vec<Object>, Vec<GeometryTableIndex>, InputSourceSpan) -> Result<GeometryTableIndex, RuntimeError>;

fn translate(interpreter: &mut Interpreter, arguments: Vec<Object>, children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<GeometryTableIndex, RuntimeError> {
    match interpreter.manifold_table.remove_many_into_union(children, span.clone())? {
        (GeometryTableEntry::Manifold(manifold), d) => {
            let (x, y, z) = accept_vec3_argument(arguments, span.clone())?;
            Ok(interpreter.manifold_table.add_manifold(manifold.translate(x, y, z), d))
        },

        (GeometryTableEntry::CrossSection(cross_section), d) => {
            let (x, y) = accept_vec2_argument(arguments, span.clone())?;
            Ok(interpreter.manifold_table.add_cross_section(cross_section.translate(x, y), d))
        },
    }
}

fn union(interpreter: &mut Interpreter, arguments: Vec<Object>, children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<GeometryTableIndex, RuntimeError> {
    reject_arguments(arguments, &span)?;

    let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span)?;
    Ok(interpreter.manifold_table.add(geom, disp))
}

fn difference(interpreter: &mut Interpreter, arguments: Vec<Object>, mut children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<GeometryTableIndex, RuntimeError> {
    reject_arguments(arguments, &span)?;

    if children.is_empty() {
        return Err(RuntimeError::new(RuntimeErrorKind::ChildrenExpected, span))
    }

    let minuend = children.remove(0);
    if children.is_empty() {
        return Ok(minuend);
    }
    
    let (subtrahend, _) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;
    match (interpreter.manifold_table.get(&minuend), subtrahend) {
        (GeometryTableEntry::Manifold(_), GeometryTableEntry::Manifold(subtrahend_manifold)) => {
            Ok(interpreter.manifold_table.map_manifold(minuend, |m| m.difference(&subtrahend_manifold)))
        },

        (GeometryTableEntry::CrossSection(_), GeometryTableEntry::CrossSection(subtrahend_cross_section)) => {
            Ok(interpreter.manifold_table.map_cross_section(minuend, |m| m.difference(&subtrahend_cross_section)))
        },

        _ => {
            Err(RuntimeError::new(RuntimeErrorKind::MixedGeometryDimensions, span))
        }
    }
}

fn linear_extrude(interpreter: &mut Interpreter, arguments: Vec<Object>, children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<GeometryTableIndex, RuntimeError> {
    let [height] = accept_arguments(arguments, &span)?;
    let height = height.as_number(span.clone())?;

    let (geom, disp) = interpreter.manifold_table.remove_many_into_union(children, span.clone())?;
    let GeometryTableEntry::CrossSection(cross_section) = geom
    else { return Err(RuntimeError::new(RuntimeErrorKind::Requires2DGeometry, span.clone())) };

    Ok(interpreter.manifold_table.add_manifold(Manifold::extrude(cross_section.polygons(), height), disp))
}

fn buffer(interpreter: &mut Interpreter, arguments: Vec<Object>, children: Vec<GeometryTableIndex>, span: InputSourceSpan) -> Result<GeometryTableIndex, RuntimeError> {
    reject_arguments(arguments, &span)?;

    let (geom, _) = interpreter.manifold_table.remove_many_into_union(children, span)?;
    Ok(interpreter.manifold_table.add(geom, GeometryDisposition::Virtual))
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
