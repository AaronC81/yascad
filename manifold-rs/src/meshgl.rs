use std::os::raw::c_void;

use crate::{Vec3, manifold::Manifold, raw};

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

            raw::manifold_meshgl_vert_properties(data.as_mut_ptr() as *mut c_void, self.ptr);
            data.set_len(length);

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

            raw::manifold_meshgl_tri_verts(data.as_mut_ptr() as *mut c_void, self.ptr);
            data.set_len(length);

            data.into_iter().map(|i| i as usize).collect()
        }
    }

    /// Returns an iterator over high-level triangle data for this mesh.
    /// 
    /// Each item includes:
    ///   - The three vertex points which define the triangle
    ///   - Any additional arbitrary vertex properties
    pub fn iter_triangles(&self) -> impl Iterator<Item = MeshTriangle> {
        let vp = self.count_vertex_properties();
        let verts = self.vertex_property_data();
        let tris = self.triangle_vertex_data();

        (0..self.count_triangles()).map(move |tri_index| {
            // Find raw property data for each vertex
            let vert_indices = &tris[(tri_index * VERTICES_IN_TRI)..(tri_index * VERTICES_IN_TRI + VERTICES_IN_TRI)];
            let vert_props: [&[f32]; VERTICES_IN_TRI] = (0..VERTICES_IN_TRI)
                .map(|i| &verts[(vert_indices[i] * vp)..(vert_indices[i] * vp + 3)])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            let mut triangle = MeshTriangle::default();
            for (i, props) in vert_props.into_iter().enumerate() {
                // First three props are always X, Y, Z, and will definitely exist
                triangle.points[i].x = props[0];
                triangle.points[i].y = props[1];
                triangle.points[i].z = props[2];

                // Future props are arbitrary
                triangle.properties[i] = props[3..].to_vec();
            }
            
            triangle
        })
    }
}

impl Drop for MeshGL {
    fn drop(&mut self) {
        unsafe {
            raw::manifold_delete_meshgl(self.ptr);
        }
    }
}

const VERTICES_IN_TRI: usize = 3;

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct MeshTriangle {
    pub points: [Vec3<f32>; VERTICES_IN_TRI],
    pub properties: [Vec<f32>; VERTICES_IN_TRI],
}
