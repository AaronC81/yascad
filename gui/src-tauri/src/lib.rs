use std::{fs::File, path::PathBuf};

use manifold_rs::ext::MeshGLExt;
use miette::Diagnostic;
use yascad_lang::{InputSource, LangError, build_model};

#[tauri::command]
fn render_preview(code: &str) -> Result<String, String> {
    let source = InputSource::new_string(code.to_owned());

    let res = build_model(source);
    println!("{res:?}");
    match res {
        Ok(model) => {
            let stl = model.meshgl().to_stl("YASCADPreview");
            let mut stl_bytes = vec![];
            stl.write_text_stl(&mut stl_bytes).unwrap();
            let stl_text = String::from_utf8(stl_bytes).unwrap();

            Ok(stl_text)
        }

        Err(LangError::Tokenize(errors)) => Err(flatten_miette_errors(errors)),
        Err(LangError::Parser(errors)) => Err(flatten_miette_errors(errors)),
        Err(LangError::Runtime(error)) => Err(flatten_miette_errors(vec![error])),
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
