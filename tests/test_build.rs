use std::error::Error;

use insta::{assert_binary_snapshot, assert_snapshot, glob, with_settings};
use manifold_rs::ext::MeshGLExt;
use yascad_lang::{InputSource, LangError, build_model};

#[test]
fn test_build() {
    let mut settings = insta::Settings::clone_current();
    settings.remove_info();
    let _settings_guard = settings.bind_to_scope();

    glob!("inputs/*.yascad", |path| {
        let source = InputSource::new_file(path).unwrap();
        let model = build_model(source).unwrap();

        let mut stl = model.meshgl().to_stl("YASCADText");
        stl.sort();
        let mut text_stl = Vec::new();
        stl.write_text_stl(&mut text_stl).unwrap();

        assert_binary_snapshot!(".stl", text_stl);
    });
}

#[test]
fn test_error() {
    fn flatten_errors<E: Error>(errors: Vec<E>) -> String {
        errors
            .into_iter()
            .map(|e| format!("{e}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    glob!("bad_inputs/*.yascad", |path| {
        let source = InputSource::new_file(path).unwrap();
        let error = build_model(source).unwrap_err();

        let errors = match error {
            LangError::Tokenize(errors) => flatten_errors(errors),
            LangError::Parser(errors) => flatten_errors(errors),
            LangError::Runtime(error) => flatten_errors(vec![error]),
        };

        assert_snapshot!(errors);
    });
}
