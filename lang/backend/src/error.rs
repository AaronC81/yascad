use std::{error::Error, fmt::Display, ops::RangeInclusive};

use yascad_frontend::InputSourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
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
    UndefinedField { ty: String, field: String },
    IncorrectArity { expected: RangeInclusive<usize>, actual: usize },
    IncorrectVectorLength { expected: RangeInclusive<usize>, actual: usize },
    MixedManifoldDisposition,
    DuplicateBinding(String),
    DuplicateOperator(String),
    ItReferenceInvalid,
    ItReferenceUnsupportedNotOneChild,
    ChildrenInvalid,
}

impl Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeErrorKind::IncorrectType { expected, actual } => write!(f, "type error - expected {expected}, got {actual}"),
            RuntimeErrorKind::UndefinedIdentifier(id) => write!(f, "undefined identifier \"{id}\""),
            RuntimeErrorKind::UndefinedField { ty, field } => write!(f, "{ty} object has no field \"{field}\""),
            RuntimeErrorKind::IncorrectArity { expected, actual } => {
                write!(f, "incorrect number of arguments - expected ")?;
                fmt_length_range(f, expected)?;
                write!(f, ", got {actual}")?;
                Ok(())
            },
            RuntimeErrorKind::IncorrectVectorLength { expected, actual } => {
                write!(f, "incorrect vector length - expected ")?;
                fmt_length_range(f, expected)?;
                write!(f, ", got {actual}")?;
                Ok(())
            },
            RuntimeErrorKind::MixedManifoldDisposition => write!(f, "this operation tried to mix manifolds of different dispositions"),
            RuntimeErrorKind::DuplicateBinding(id) => write!(f, "binding named \"{id}\" is already defined"),
            RuntimeErrorKind::DuplicateOperator(id) => write!(f, "operator named \"{id}\" is already defined"),
            RuntimeErrorKind::ItReferenceInvalid => write!(f, "cannot use `it` outside of operator target arguments"),
            RuntimeErrorKind::ItReferenceUnsupportedNotOneChild => write!(f, "`it` is not currently supported without exactly one operator child - consider using `union()` first"),
            RuntimeErrorKind::ChildrenInvalid => write!(f, "cannot use `children` outside of operator body"),
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
