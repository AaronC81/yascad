use std::{path::PathBuf, rc::Rc};

use yascad_frontend::InputSource;

#[tauri::command]
fn render_preview(code: &str) -> Result<String, String> {
    let source = Rc::new(InputSource::new_string(code.to_owned()));

    let (tokens, errors) = yascad_frontend::tokenize(source.clone());
    if !errors.is_empty() {
        return Err(
            errors.into_iter()
                .map(|e| format!("{:?}", miette::Report::new(e)))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    let mut parser = yascad_frontend::Parser::new(source.clone(), tokens);
    let stmts = parser.parse_statements();

    if !parser.errors.is_empty() {
        return Err(
            parser.errors.into_iter()
                .map(|e| format!("{:?}", miette::Report::new(e)))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    let mut interpreter = yascad_backend::Interpreter::new();
    match interpreter.interpret_top_level(&stmts) {
        Ok(_) => {
            // TODO: use unique temp file paths (multiple instances, cross-platform)
            let temp_path = PathBuf::from("/tmp/yascad_model.stl");

            interpreter
                .build_top_level_manifold()
                .meshgl()
                .export(&temp_path);
            Ok(temp_path.to_string_lossy().to_string())
        },
        Err(error) => {
            Err(
                errors.into_iter()
                    .map(|e| format!("{e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![render_preview])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
