use wasm_bindgen::prelude::wasm_bindgen;
use yascad_lang::InputSource;

#[wasm_bindgen]
pub fn yascad_test() -> String {
    match yascad_lang::build_model(InputSource::new_string("cube(1);".to_owned())) {
        Ok(manifold) => format!("Wahey, tris {}", manifold.num_tri()),
        Err(e) => format!("Aww, {e:?}"),
    }
}
