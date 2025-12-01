use crate::{MeshGL, Vec3, ext::Stl};

/// Extends [`MeshGL`] with methods not originally from Manifold.
pub trait MeshGLExt {
    /// Convert this mesh to an STL.
    fn to_stl(&self, name: &str) -> Stl;
}

impl MeshGLExt for MeshGL {
    fn to_stl(&self, name: &str) -> Stl {
        let mut stl = Stl::new(name);

        for tri in self.iter_triangles() {
            let normal = triangle_normal(tri.points[0], tri.points[1], tri.points[2]);
            stl.add_triangle(normal, tri.points);
        }

        stl
    }
}

fn triangle_normal(p1: Vec3<f32>, p2: Vec3<f32>, p3: Vec3<f32>) -> Vec3<f32> {
    // Edge vectors
    let u = p2 - p1;
    let v = p3 - p1;

    // Cross product u × v
    let normal = [
        u.y * v.z - u.z * v.y,
        u.z * v.x - u.x * v.z,
        u.x * v.y - u.y * v.x,
    ];

    // Normalize
    let len = (normal[0]*normal[0] + normal[1]*normal[1] + normal[2]*normal[2]).sqrt();
    if len > 0.0 {
        Vec3::new(normal[0] / len, normal[1] / len, normal[2] / len)
    } else {
        Vec3::new(0.0, 0.0, 0.0) // Degenerate triangle
    }
}
