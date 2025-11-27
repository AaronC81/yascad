use std::{alloc::{Layout, alloc}, ffi::CString, os::raw::c_void};

mod raw {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

fn main() {
    println!("Hello, world!");

    unsafe {
        let cube_mem = raw::manifold_alloc_manifold();
        let cube = raw::manifold_cube(cube_mem as *mut c_void, 1.0, 1.0, 5.0, 0);

        let mesh_mem = raw::manifold_alloc_meshgl();
        let mesh = raw::manifold_get_meshgl(mesh_mem as *mut c_void, cube);

        // No idea on alignment. 8 is a safe bet?
        let options_mem = alloc(Layout::from_size_align(raw::manifold_export_options_size(), 8).unwrap()) as *mut c_void;
        let options = raw::manifold_export_options(options_mem);

        let filename = CString::new("rust.stl").unwrap();
        raw::manifold_export_meshgl(filename.as_ptr(), mesh, options);
    }

    println!("Done");
}

