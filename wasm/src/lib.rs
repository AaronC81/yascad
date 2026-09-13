use manifold_csg_ext::MeshGLExt;
use miette::{Diagnostic, Report};
use wasm_bindgen::prelude::wasm_bindgen;
use yascad_lang::{BuildModelOptions, InputSource, LangError, build_model};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}

#[wasm_bindgen(js_name = "buildYascadModelToStl")]
pub fn build_yascad_model_to_stl(code: String) -> Result<String, String> {
    let source = InputSource::new_string(code.to_owned());

    let res = build_model(source, BuildModelOptions {
        debug_hook: Some(Box::new(|o| {
            console_log(&format!("{o:?}"));
        })),
        ..Default::default()
    });
    match res {
        Ok(model) => {
            let stl = model.to_meshgl().to_stl("YASCADPreview");
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

// TODO: return something that implements `Into<JsValue>` so the GUI can get structured errors.
// Could render them nicely in the editor.
fn flatten_miette_errors<E: Diagnostic + Send + Sync + 'static>(errors: Vec<E>) -> String {
    errors
        .into_iter()
        .map(|e| format!("{:?}", Report::new(e)))
        .collect::<Vec<_>>()
        .join("\n")
}
