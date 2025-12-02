use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // Manifold needs a libc++ to link against
    println!("cargo:rustc-link-lib=c++");

    // Build our own dependencies - configuration is in CMakeLists
    CMakeWrapper(cmake::build(".."))
        .static_lib_dependency("manifoldc", "build/vendor/manifold/bindings/c")
        .static_lib_dependency("manifold",  "build/vendor/manifold/src")
        .static_lib_dependency("Clipper2",  "build/_deps/clipper2-build");

    // Generate bindings for Manifold's C interface
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

struct CMakeWrapper(PathBuf);
impl CMakeWrapper {
    /// Helper to emit a static library search path and dependency from the cmake build path
    fn static_lib_dependency(&mut self, lib: &str, relative_path: impl AsRef<Path>) -> &mut Self {
        println!("cargo:rustc-link-search={}", self.0.join(relative_path).display());
        println!("cargo:rustc-link-lib=static={lib}");
        self
    }
}
