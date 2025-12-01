use crate::{MeshGL, Vec3, ext::Stl};

/// Extends [`MeshGL`] with methods not originally from Manifold.
pub trait MeshGLExt {
    /// Convert this mesh to an STL.
    fn to_stl(&self, name: &str) -> Stl;
}

impl MeshGLExt for MeshGL {
    fn to_stl(&self, name: &str) -> Stl {
        let mut stl = Stl::new(name);

        let vp = self.count_vertex_properties();

        // TODO: nice iterators for these
        let verts = self.vertex_property_data();
        let tris = self.triangle_vertex_data();

        for tri_index in 0..self.count_triangles() {
            let pt1_vert_index = tris[tri_index * 3    ];
            let pt2_vert_index = tris[tri_index * 3 + 1];
            let pt3_vert_index = tris[tri_index * 3 + 2];

            let pt1_vert_data = &verts[(pt1_vert_index * vp)..(pt1_vert_index * vp + 3)];
            let pt2_vert_data = &verts[(pt2_vert_index * vp)..(pt2_vert_index * vp + 3)];
            let pt3_vert_data = &verts[(pt3_vert_index * vp)..(pt3_vert_index * vp + 3)];

            let normal = triangle_normal(pt1_vert_data, pt2_vert_data, pt3_vert_data);

            let pt1_vec = Vec3::<f32>::new(pt1_vert_data[0], pt1_vert_data[1], pt1_vert_data[2]);
            let pt2_vec = Vec3::<f32>::new(pt2_vert_data[0], pt2_vert_data[1], pt2_vert_data[2]);
            let pt3_vec = Vec3::<f32>::new(pt3_vert_data[0], pt3_vert_data[1], pt3_vert_data[2]);

            stl.add_triangle(normal, [pt1_vec, pt2_vec, pt3_vec]);
        }

        stl
    }
}

// Temporary LLM special
fn triangle_normal(p1: &[f32], p2: &[f32], p3: &[f32]) -> Vec3<f32> {
    // Edge vectors
    let u = [
        p2[0] - p1[0],
        p2[1] - p1[1],
        p2[2] - p1[2],
    ];

    let v = [
        p3[0] - p1[0],
        p3[1] - p1[1],
        p3[2] - p1[2],
    ];

    // Cross product u × v
    let normal = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];

    // Normalize
    let len = (normal[0]*normal[0] + normal[1]*normal[1] + normal[2]*normal[2]).sqrt();
    if len > 0.0 {
        Vec3::new(normal[0] / len, normal[1] / len, normal[2] / len)
    } else {
        Vec3::new(0.0, 0.0, 0.0) // Degenerate triangle
    }
}
