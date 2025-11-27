use std::{alloc::{Layout, alloc}, ffi::CString, os::raw::c_void, path::Path};

use crate::{manifold::Manifold, raw};

pub struct MeshGL {
    ptr: *mut raw::ManifoldMeshGL,
}

impl MeshGL {
    /// Take ownership of an already-allocated mesh pointer.
    /// 
    /// Safety: The pointer must be valid, unique, and point to an allocated manifold instance.
    /// The instance must be initialised, if not already, before further use.
    unsafe fn from_raw(ptr: *mut raw::ManifoldMeshGL) -> Self {
        Self { ptr }
    }

    /// Allocate an empty mesh.
    /// 
    /// Safety: This is actually safe, as the FFI interface allocates a valid empty mesh.
    /// However, further initialisation is probably intended. If you really want an empty mesh,
    /// use [`MeshGL::new`].
    unsafe fn alloc() -> Self {
        unsafe {
            Self::from_raw(raw::manifold_alloc_meshgl())
        }
    }

    /// Create an empty mesh.
    pub fn new() -> Self {
        unsafe { Self::alloc() }
    }

    /// Create a mesh for a given manifold.
    pub fn from_manifold(manifold: &Manifold) -> Self {
        unsafe {
            let mesh = Self::alloc();
            raw::manifold_get_meshgl(mesh.ptr as *mut c_void, manifold.ptr);
            mesh
        }
    }

    /// Export this mesh to a file.
    pub fn export(&self, path: impl AsRef<Path>) {
        // TODO: the provided `manifold_export_meshgl` is apparently "provided as a demonstration"
        //       and the C bindings for it don't seem to give me any way to handle errors.
        //       Maybe this needs replacing with something different/custom.

        // TODO: permit taking these options.
        // Not sure how you're supposed to allocate this.
        unsafe {
            // Alignment of 8 is a safe guess
            let options = alloc(Layout::from_size_align(raw::manifold_export_options_size(), 8).unwrap()) as *mut raw::ManifoldExportOptions;
            raw::manifold_export_options(options as *mut c_void);

            let path = CString::new(path.as_ref().as_os_str().as_encoded_bytes()).unwrap();
            raw::manifold_export_meshgl(path.as_ptr(), self.ptr, options);
        }
    }
}

impl Drop for MeshGL {
    fn drop(&mut self) {
        unsafe {
            raw::manifold_delete_meshgl(self.ptr);
        }
    }
}
