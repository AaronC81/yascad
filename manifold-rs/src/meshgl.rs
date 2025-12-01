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

    /// Get the number of vertices in this mesh.
    pub fn count_vertices(&self) -> usize {
        unsafe {
            raw::manifold_meshgl_num_vert(self.ptr) as usize
        }
    }

    /// Get the number of triangles in this mesh.
    pub fn count_triangles(&self) -> usize {
        unsafe {
            raw::manifold_meshgl_num_tri(self.ptr) as usize
        }
    }

    /// The number of properties which each vertex has.
    /// 
    /// This is the stride for [`Self::vertex_property_data`].
    pub fn count_vertex_properties(&self) -> usize {
        unsafe {
            raw::manifold_meshgl_num_prop(self.ptr) as usize
        }
    }

    /// Gets raw vertex properties for this mesh.
    /// 
    /// All vertices have the same number of properties, but that could be any number >= 3.
    /// The first three properties are the x/y/z coordinates of the vertex.
    /// 
    /// Assuming no extra properties, the data looks like:
    /// 
    /// ```text
    /// [v1.x, v1.y, v1.z, v2.x, v2.y, v2.z, ...]
    /// ```
    pub fn vertex_property_data(&self) -> Vec<f32> {
        unsafe {
            let length = raw::manifold_meshgl_vert_properties_length(self.ptr);
            let mut data = Vec::<f32>::with_capacity(length);
            data.set_len(length);

            raw::manifold_meshgl_vert_properties(data.as_mut_ptr() as *mut c_void, self.ptr);

            data
        }
    }

    /// Gets vertex indices which compose the triangles in this mesh.
    /// 
    /// Each element is a vertex index. You can look up the vertex in [`Self::vertex_property_data`]
    /// by multiplying it with the stride specified by [`Self::count_vertex_properties`].
    /// 
    /// Each triangle is a set of 3 elements in this vector, in counter-clockwise (from outside)
    /// order:
    /// 
    /// ```text
    /// [t1.1, t1.2, t1.3, t2.1, t2.2, t2.3, ...]
    /// ```
    pub fn triangle_vertex_data(&self) -> Vec<usize> {
        unsafe {
            // Interface hard-codes the indices as u32.
            // We'll convert to usize later for convenience of indexing.
            let length = self.count_triangles() * 3;
            let mut data = Vec::<u32>::with_capacity(length);
            data.set_len(length);

            raw::manifold_meshgl_tri_verts(data.as_mut_ptr() as *mut c_void, self.ptr);

            data.into_iter().map(|i| i as usize).collect()
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
