use std::{error::Error, fmt::Display, ops::RangeInclusive};

use miette::Diagnostic;
use yascad_frontend::InputSourceSpan;

#[derive(Debug, Clone, PartialEq, Eq, Diagnostic)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,

    #[source_code]
    #[label]
    pub span: InputSourceSpan,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, span: InputSourceSpan) -> Self {
        Self { kind, span }
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: emit file and line number, but need to have a way to work out line number first
        write!(f, "{}", self.kind)
    }
}
impl Error for RuntimeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    IncorrectType { expected: String, actual: String },
    UndefinedIdentifier(String),
    InvalidIdentifier { id: String, kind: String },
    UndefinedField { ty: String, field: String },
    IncorrectArity { expected: RangeInclusive<usize>, actual: usize },
    DuplicateNamedArgument(String),
    UndefinedNamedArgument(String),
    MissingNamedArguments(Vec<String>),
    NamedArgumentRepeatsPositionalArgument(String),
    IncorrectVectorLength { expected: RangeInclusive<usize>, actual: usize },
    MixedGeometryDisposition,
    MixedGeometryDimensions,
    DuplicateName(String),
    ItReferenceInvalid,
    ItReferenceUnsupportedNotOneChild,
    ChildrenExpected,
    ChildrenInvalid,
    FlippedRange,
    Requires2DGeometry,
}

impl Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeErrorKind::IncorrectType { expected, actual } => write!(f, "type error - expected {expected}, got {actual}"),
            RuntimeErrorKind::UndefinedIdentifier(id) => write!(f, "undefined identifier \"{id}\""),
            RuntimeErrorKind::InvalidIdentifier { id, kind } => write!(f, "identifier \"{id}\" is a {kind}, which cannot be used here"),
            RuntimeErrorKind::UndefinedField { ty, field } => write!(f, "{ty} object has no field \"{field}\""),
            RuntimeErrorKind::IncorrectArity { expected, actual } => {
                write!(f, "incorrect number of positional arguments - expected ")?;
                fmt_length_range(f, expected)?;
                write!(f, ", got {actual}")?;
                Ok(())
            },
            RuntimeErrorKind::DuplicateNamedArgument(name) => write!(f, "argument \"{name}\" cannot be passed by name more than once"),
            RuntimeErrorKind::UndefinedNamedArgument(name) => write!(f, "no argument named \"{name}\""),
            RuntimeErrorKind::MissingNamedArguments(names) => {
                if names.len() > 1 {
                    write!(f, "missing multiple arguments: {}", names.join(", "))
                } else {
                    write!(f, "missing argument \"{}\"", names[0])
                }
            }
            RuntimeErrorKind::NamedArgumentRepeatsPositionalArgument(name) => write!(f, "argument \"{name}\" has already been passed as a positional argument, so cannot be passed again by name"),
            RuntimeErrorKind::IncorrectVectorLength { expected, actual } => {
                write!(f, "incorrect vector length - expected ")?;
                fmt_length_range(f, expected)?;
                write!(f, ", got {actual}")?;
                Ok(())
            },
            RuntimeErrorKind::MixedGeometryDisposition => write!(f, "this operation tried to mix geometries of different dispositions"),
            RuntimeErrorKind::MixedGeometryDimensions => write!(f, "this operation tried to mix 2D and 3D geometry"),
            RuntimeErrorKind::DuplicateName(id) => write!(f, "name \"{id}\" is already defined"),
            RuntimeErrorKind::ItReferenceInvalid => write!(f, "cannot use `it` outside of operator target arguments"),
            RuntimeErrorKind::ItReferenceUnsupportedNotOneChild => write!(f, "`it` is not currently supported without exactly one operator child - consider using `union()` first"),
            RuntimeErrorKind::ChildrenInvalid => write!(f, "cannot use `children` outside of operator body"),
            RuntimeErrorKind::ChildrenExpected => write!(f, "this operation requires at least one child"),
            RuntimeErrorKind::FlippedRange => write!(f, "end of range is lower than start"),
            RuntimeErrorKind::Requires2DGeometry => write!(f, "this operation requires 2D geometry, but 3D was provided"),
        }
    }
}

fn fmt_length_range(f: &mut std::fmt::Formatter<'_>, range: &RangeInclusive<usize>) -> std::fmt::Result {
    if range.start() == range.end() {
        write!(f, "{}", range.start())
    } else {
        write!(f, "{}-{}", range.start(), range.end())
    }
}
