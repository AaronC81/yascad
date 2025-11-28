use std::rc::Rc;

use yascad_backend::Interpreter;
use yascad_frontend::{InputSource, Parser, tokenize};

fn main() {
    let source = Rc::new(InputSource::new_file("model.yascad").unwrap());

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
        .export("out.stl");
}
