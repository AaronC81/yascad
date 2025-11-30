use std::path::PathBuf;

use miette::Diagnostic;
use yascad_lang::{InputSource, LangError, build_model};

#[tauri::command]
fn render_preview(code: &str) -> Result<String, String> {
    let source = InputSource::new_string(code.to_owned());

    match build_model(source) {
        Ok(model) => {
            // TODO: use unique temp file paths (multiple instances, cross-platform)
            let temp_path = PathBuf::from("/tmp/yascad_model.stl");
            model.meshgl().export(&temp_path);

            Ok(temp_path.to_string_lossy().to_string())
        }

        Err(LangError::Tokenize(errors)) => Err(flatten_miette_errors(errors)),
        Err(LangError::Parser(errors)) => Err(flatten_miette_errors(errors)),
        Err(LangError::Runtime(error)) => Err(format!("{error}")),
    }
}

fn flatten_miette_errors<E: Diagnostic + Send + Sync + 'static>(errors: Vec<E>) -> String {
    errors.into_iter()
        .map(|e| format!("{:?}", miette::Report::new(e)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![render_preview])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
