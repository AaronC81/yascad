use std::{path::PathBuf, process::exit, rc::Rc};

use clap::Parser as ClapParser;
use miette::Diagnostic;
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
    if !errors.is_empty() {
        abort_with_errors(errors);
    }

    let mut parser = Parser::new(source.clone(), tokens);
    let stmts = parser.parse_statements();

    if !parser.errors.is_empty() {
        abort_with_errors(parser.errors);
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

fn abort_with_errors<E: Diagnostic + Send + Sync + 'static>(errors: Vec<E>) -> ! {
    for error in errors {
        println!("{:?}", miette::Report::new(error));
    }
    exit(1);
}
