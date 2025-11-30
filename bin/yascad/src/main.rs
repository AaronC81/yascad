use std::{path::PathBuf, process::exit};

use clap::Parser as ClapParser;
use miette::Diagnostic;
use yascad_lang::{InputSource, LangError, build_model};

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
    let source = InputSource::new_file(args.input).unwrap();

    match build_model(source) {
        Ok(model) => {
            model.meshgl().export(args.output);
        }

        Err(LangError::Tokenize(errors)) => abort_with_errors(errors),
        Err(LangError::Parser(errors)) => abort_with_errors(errors),
        Err(LangError::Runtime(error)) => {
            println!("{error}");
            return;
        }
    }
}

fn abort_with_errors<E: Diagnostic + Send + Sync + 'static>(errors: Vec<E>) -> ! {
    for error in errors {
        println!("{:?}", miette::Report::new(error));
    }
    exit(1);
}
