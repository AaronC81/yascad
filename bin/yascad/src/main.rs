use std::{path::PathBuf, rc::Rc};

use clap::Parser as ClapParser;
use yascad_backend::Interpreter;
use yascad_frontend::{InputSource, Parser, tokenize};

#[derive(ClapParser, Debug)]
struct Args {
    /// Path to the input file
    #[arg(short)]
    input: PathBuf,

    /// Path to the output file
    #[arg(short)]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();

    let source = Rc::new(InputSource::new_file(args.input).unwrap());

    let (tokens, errors) = tokenize(source.clone());
    assert!(errors.is_empty());

    let mut parser = Parser::new(source.clone(), tokens);
    let stmts = parser.parse_statements();

    if !parser.errors.is_empty() {
        for error in parser.errors {
            println!("{error}");
        }
        return;
    }

    let mut interpreter = Interpreter::new();
    for stmt in stmts {
        match interpreter.interpret_top_level(&stmt) {
            Ok(_) => {},
            Err(error) => {
                println!("{error}");
                return;
            }
        }
    }

    interpreter
        .build_top_level_manifold()
        .meshgl()
        .export(args.output);
}
