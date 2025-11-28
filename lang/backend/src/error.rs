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
    IncorrectArity { expected: RangeInclusive<usize>, actual: usize },
    MixedManifoldDisposition,
}

impl Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeErrorKind::IncorrectType { expected, actual } => write!(f, "type error - expected {expected}, got {actual}"),
            RuntimeErrorKind::UndefinedIdentifier(id) => write!(f, "undefined identifier \"{id}\""),
            RuntimeErrorKind::IncorrectArity { expected, actual } => {
                write!(f, "incorrect number of arguments - expected ")?;

                if expected.start() == expected.end() {
                    write!(f, "{}", expected.start())?;
                } else {
                    write!(f, "{}-{}", expected.start(), expected.end())?;
                }

                write!(f, ", got {actual}")?;
                Ok(())
            },
            RuntimeErrorKind::MixedManifoldDisposition => write!(f, "this operation tried to mix manifolds of different dispositions"),
        }
    }
}
