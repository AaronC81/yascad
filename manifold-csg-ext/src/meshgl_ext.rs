use super::stl::Stl;
use manifold_csg::MeshGL;

const VERTICES_IN_TRI: usize = 3;

#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub struct MeshTriangle {
    pub points: [[f32; 3]; VERTICES_IN_TRI],
    pub properties: [Vec<f32>; VERTICES_IN_TRI],
}

/// Extends [`MeshGL`] with methods not originally from Manifold.
pub trait MeshGLExt {
    /// Returns an iterator over high-level triangle data for this mesh.
    /// 
    /// Each item includes:
    ///   - The three vertex points which define the triangle
    ///   - Any additional arbitrary vertex properties
    fn iter_triangles(&self) -> impl Iterator<Item = MeshTriangle>;

    /// Convert this mesh to an STL.
    fn to_stl(&self, name: &str) -> Stl;
}

impl MeshGLExt for MeshGL {
    /// Returns an iterator over high-level triangle data for this mesh.
    /// 
    /// Each item includes:
    ///   - The three vertex points which define the triangle
    ///   - Any additional arbitrary vertex properties
    fn iter_triangles(&self) -> impl Iterator<Item = MeshTriangle> {
        let vp = self.num_prop();
        let verts = self.vert_properties();
        let tris = self.tri_verts();

        (0..self.num_tri()).map(move |tri_index| {
            // Find raw property data for each vertex
            let vert_indices = &tris[(tri_index * VERTICES_IN_TRI)..(tri_index * VERTICES_IN_TRI + VERTICES_IN_TRI)];
            let vert_props: [&[f32]; VERTICES_IN_TRI] = (0..VERTICES_IN_TRI)
                .map(|i| &verts[(vert_indices[i] as usize * vp)..(vert_indices[i] as usize * vp + 3)])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            let mut triangle = MeshTriangle::default();
            for (i, props) in vert_props.into_iter().enumerate() {
                // First three props are always X, Y, Z, and will definitely exist
                triangle.points[i][0] = props[0];
                triangle.points[i][1] = props[1];
                triangle.points[i][2] = props[2];

                // Future props are arbitrary
                triangle.properties[i] = props[3..].to_vec();
            }
            
            triangle
        })
    }

    fn to_stl(&self, name: &str) -> Stl {
        let mut stl = Stl::new(name);

        for tri in self.iter_triangles() {
            let normal = triangle_normal(tri.points[0], tri.points[1], tri.points[2]);
            stl.add_triangle(normal, tri.points);
        }

        stl
    }
}

fn triangle_normal(p1: [f32; 3], p2: [f32; 3], p3: [f32; 3]) -> [f32; 3] {
    // Edge vectors
    let u = vec3_sub(p2, p1);
    let v = vec3_sub(p3, p1);

    // Cross product u × v
    let normal = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];

    // Normalize
    let len = (normal[0]*normal[0] + normal[1]*normal[1] + normal[2]*normal[2]).sqrt();
    if len > 0.0 {
        [normal[0] / len, normal[1] / len, normal[2] / len]
    } else {
        [0.0, 0.0, 0.0] // Degenerate triangle
    }
}

fn vec3_sub([x1, y1, z1]: [f32; 3], [x2, y2, z2]: [f32; 3]) -> [f32; 3] {
    [x1 - x2, y1 - y2, z1 - z2]
}
