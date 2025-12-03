#![feature(type_alias_impl_trait)]

mod object;
mod geometry_table;
mod lexical_scope;

mod error;
pub use error::*;

mod builtin;

mod interpreter;
pub use interpreter::*;
