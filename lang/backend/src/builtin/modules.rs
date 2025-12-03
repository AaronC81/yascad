use manifold_rs::{CrossSection, Manifold};
use yascad_frontend::InputSourceSpan;

use crate::{Interpreter, RuntimeError, RuntimeErrorKind, builtin::{accept_arguments, accept_vec2_argument, accept_vec3_argument, reject_arguments}, geometry_table::{GeometryDisposition, GeometryTableIndex}, object::Object};

pub type ModuleDefinition = &'static dyn Fn(&mut Interpreter, Vec<Object>, Option<&[GeometryTableIndex]>, InputSourceSpan) -> Result<Object, RuntimeError>;

fn cube(interpreter: &mut Interpreter, arguments: Vec<Object>, _operator_children: Option<&[GeometryTableIndex]>, span: InputSourceSpan) -> Result<Object, RuntimeError> {
    let (x, y, z) = accept_vec3_argument(arguments, span)?;
    Ok(Object::Manifold(interpreter.manifold_table.add_manifold(Manifold::cube(x, y, z, false), GeometryDisposition::Physical)))
}

fn cylinder(interpreter: &mut Interpreter, arguments: Vec<Object>, _operator_children: Option<&[GeometryTableIndex]>, span: InputSourceSpan) -> Result<Object, RuntimeError> {
    // TODO: needs to support diameters or cone forms
    let [height, radius] = accept_arguments(arguments, &span)?;
    let height = height.as_number(span.clone())?;
    let radius = radius.as_number(span.clone())?;

    Ok(Object::Manifold(interpreter.manifold_table.add_manifold(Manifold::cylinder(radius, height, interpreter.circle_segments, false), GeometryDisposition::Physical)))
}

fn square(interpreter: &mut Interpreter, arguments: Vec<Object>, _operator_children: Option<&[GeometryTableIndex]>, span: InputSourceSpan) -> Result<Object, RuntimeError> {
    let (x, y) = accept_vec2_argument(arguments, span)?;
    Ok(Object::Manifold(interpreter.manifold_table.add_cross_section(CrossSection::square(x, y, false), GeometryDisposition::Physical)))
}

fn copy(interpreter: &mut Interpreter, arguments: Vec<Object>, _operator_children: Option<&[GeometryTableIndex]>, span: InputSourceSpan) -> Result<Object, RuntimeError> {
    let [manifold_index] = accept_arguments(arguments, &span)?;
    let manifold_index = manifold_index.into_manifold(span)?;
    let manifold = interpreter.manifold_table.get(&manifold_index);

    // Even if it's being copied in a virtual disposition, we can make it physical here.
    // The `buffer` will "downgrade" it later.
    let copied_manifold = interpreter.manifold_table.add(manifold.clone(), GeometryDisposition::Physical);
    Ok(Object::Manifold(copied_manifold))
}

fn children(interpreter: &mut Interpreter, arguments: Vec<Object>, operator_children: Option<&[GeometryTableIndex]>, span: InputSourceSpan) -> Result<Object, RuntimeError> {
    reject_arguments(arguments, &span)?;

    let Some(children) = operator_children
    else {
        return Err(RuntimeError::new(RuntimeErrorKind::ChildrenInvalid, span));
    };

    // The children are temporary virtual manifolds.
    // Copy them as physical and then build a union of all of the copies.
    let copied_children = children.iter()
        .map(|child| {
            let m = interpreter.manifold_table.get(child).clone();
            interpreter.manifold_table.add(m, GeometryDisposition::Physical)
        })
        .collect::<Vec<_>>();

    let (geom, disp) = interpreter.manifold_table.remove_many_into_union(copied_children, span)?;
    Ok(interpreter.manifold_table.add_into_object(geom, disp))
}

fn __debug(_interpreter: &mut Interpreter, arguments: Vec<Object>, _operator_children: Option<&[GeometryTableIndex]>, _span: InputSourceSpan) -> Result<Object, RuntimeError> {
    println!("{arguments:#?}");
    Ok(Object::Null)
}

/// Get the implementation for a specific built-in module.
/// 
/// Returns [`None`] if no such module exists.
pub fn get_builtin_module(name: &str) -> Option<ModuleDefinition> {
    match name {
        "cube" => Some(&cube),
        "cylinder" => Some(&cylinder),
        "square" => Some(&square),
        "copy" => Some(&copy),
        "children" => Some(&children),
        "__debug" => Some(&__debug),

        _ => None,
    }
}
