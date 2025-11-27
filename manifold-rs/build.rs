use std::env;
use std::path::PathBuf;

fn main() {
    // TODO: abs paths
    println!("cargo:rustc-link-search=/Users/aaron/Source/yascad/build/PUBLIC/bindings/c");
    println!("cargo:rustc-link-lib=manifoldc");

    let bindings = bindgen::Builder::default()
        .header("../vendor/manifold/bindings/c/include/manifold/manifoldc.h")
        .clang_arg("-I../vendor/manifold/bindings/c/include")
        .clang_arg("-DMANIFOLD_EXPORT")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
