use std::{io, path::{Path, PathBuf}, rc::Rc};

mod tokenize;
pub use tokenize::*;

mod parse;
pub use parse::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputSource {
    content: String,
    origin: InputSourceOrigin,
}

impl InputSource {
    pub fn new_string(content: String) -> Self {
        Self {
            content,
            origin: InputSourceOrigin::new_string(),
        }
    }

    pub fn new_file(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let path_buf = path.as_ref().to_owned();
        let content = std::fs::read_to_string(path)?;
        Ok(Self {
            content,
            origin: InputSourceOrigin::File(path_buf),
        })
    } 

    pub fn span(self: &Rc<Self>, start: usize, length: usize) -> InputSourceSpan {
        InputSourceSpan::new(self.clone(), start, length)
    }

    pub fn eof_span(self: &Rc<Self>) -> InputSourceSpan {
        InputSourceSpan::new_eof(self.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputSourceOrigin {
    String,
    File(PathBuf),
}

impl InputSourceOrigin {
    pub fn new_string() -> Self {
        InputSourceOrigin::String
    }

    pub fn new_file(path: impl AsRef<Path>) -> Self {
        InputSourceOrigin::File(path.as_ref().to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputSourceSpan {
    pub source: Rc<InputSource>,
    pub start: usize,
    pub length: usize,
}

impl InputSourceSpan {
    pub fn new(source: Rc<InputSource>, start: usize, length: usize) -> Self {
        Self { source, start, length }
    }

    pub fn new_eof(source: Rc<InputSource>) -> Self {
        let len = source.content.len();
        Self::new(source, len, 0)
    }

    /// Last index covered by the span
    pub fn end(&self) -> usize {
        self.start + self.length - 1
    }

    /// Create a new span which covers all of the given spans.
    /// 
    /// Panics if some spans have different sources.
    /// Returns [`None`] if the list of spans is empty.
    pub fn union(spans: &[InputSourceSpan]) -> Option<Self> {
        if spans.len() >= 2 {
            for i in 1..spans.len() {
                if !Rc::ptr_eq(&spans[0].source, &spans[i].source) {
                    panic!("spans do not all have the same source")
                }
            }
        }
        let source = spans.first()?.source.clone();

        let start = spans.iter().map(|s| s.start).min()?;
        let end = spans.iter().map(|s| s.end()).max()?;

        let length = end - start + 1;
        Some(Self::new(source, start, length))
    }

    /// Create a new span which covers this and all of the other given spans.
    /// 
    /// Panics if some spans have different sources.
    pub fn union_with(&self, spans: &[InputSourceSpan]) -> Self {
        let mut all_spans = spans.to_vec();
        all_spans.push(self.clone());

        Self::union(&all_spans[..]).unwrap() // There will always be at least one because we have ourself
    }
}
