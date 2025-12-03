use yascad_frontend::InputSourceSpan;

use crate::{RuntimeError, RuntimeErrorKind, object::Object};

/// Accept the given number of arguments, unpacking them into an array for convenient
/// destructuring.
/// 
/// Returns a [`RuntimeErrorKind::IncorrectArity`] if the number of arguments is not expected.
/// 
/// TODO: Need a form for variable numbers of arguments
pub(crate) fn accept_arguments<const N: usize>(arguments: Vec<Object>, span: &InputSourceSpan) -> Result<[Object; N], RuntimeError> {
    let actual = arguments.len();

    arguments.try_into()
        .map_err(|_| RuntimeError::new(RuntimeErrorKind::IncorrectArity { expected: N..=N, actual }, span.clone()))
}

/// Accept a single argument which is a 3D vector.
pub(crate) fn accept_vec3_argument(arguments: Vec<Object>, span: InputSourceSpan) -> Result<(f64, f64, f64), RuntimeError> {
    let [argument] = accept_arguments(arguments, &span)?;
    argument.into_3d_vector(span)
}

/// Accept a single argument which is a 2D vector.
pub(crate) fn accept_vec2_argument(arguments: Vec<Object>, span: InputSourceSpan) -> Result<(f64, f64), RuntimeError> {
    let [argument] = accept_arguments(arguments, &span)?;
    argument.into_2d_vector(span)
}
