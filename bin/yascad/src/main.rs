use std::rc::Rc;

use yascad_backend::Interpreter;
use yascad_frontend::{InputSource, Parser, tokenize};

fn main() {
    let source = Rc::new(InputSource::new_string("
        translate(20.0, 20.0, 20.0)
        {
            cube(10, 20.5, 30);
            cube(5, 5, 50);
        };

        cube(5, 5, 5);
    ".to_owned()));
    let (tokens, errors) = tokenize(source.clone());
    assert!(errors.is_empty());

    let mut parser = Parser::new(source.clone(), tokens);
    let stmts = parser.parse_statements();

    let mut interpreter = Interpreter::new();
    for stmt in stmts {
        interpreter.interpret_top_level(&stmt).unwrap();
    }

    interpreter
        .build_top_level_manifold()
        .meshgl()
        .export("out.stl");
}
