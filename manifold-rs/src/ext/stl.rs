use std::io;

use crate::Vec3;

/// An STL model.
#[derive(Debug, Clone, PartialEq)]
pub struct Stl {
    name: String,
    triangles: Vec<StlTriangle>,
}

/// A single triangle, defined by three points and a normal, in an STL model.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct StlTriangle {
    normal: Vec3<f32>,
    points: [Vec3<f32>; 3],
}

impl Stl {
    /// Create a blank model. STL models require a name, but this has no semantic effect.
    pub fn new(name: &str) -> Self {
        Stl {
            name: name.to_owned(),
            triangles: vec![],
        }
    }

    /// Add a triangle to the model.
    pub fn add_triangle(&mut self, normal: Vec3<f32>, points: [Vec3<f32>; 3]) {
        self.triangles.push(StlTriangle { normal, points });
    }

    /// Sort the model's triangles, and apply a consistent order to the vertices within the triangles.
    /// 
    /// By default, triangles are emitted in order of addition.
    /// If the source of the triangles is unstable but you'd like a diffable STL, you can sort them.
    /// The order of triangles has no effect on the STL behaviour.
    pub fn sort(&mut self) {
        self.triangles.sort_by(|a, b| a.partial_cmp(b).expect("NaN not permitted in triangle"));

        for tri in &mut self.triangles {
            // For a triangle as defined by the STL format, points ordered like 1 2 3 are equivalent to
            // 3 1 2 and 2 3 1. I think the winding needs to stay constant, so other orderings aren't
            // identical - it needs to be a rotation.
            //
            // To achieve a consistent ordering, rotate the points so that the 'lowest' one (as
            // defined by PartialOrd) is at the beginning.
            //
            // We need to think of a consistent way to order these. Manifold isn't consistent itself.
            let mut min_point_index = 0;
            for point_index in 1..=2 {
                if     tri.points[point_index].x < tri.points[min_point_index].x
                    || tri.points[point_index].y < tri.points[min_point_index].y
                    || tri.points[point_index].z < tri.points[min_point_index].z
                {
                    min_point_index = point_index;
                }
            };

            tri.points = match min_point_index {
                0 => tri.points, // Unchanged
                1 => [tri.points[1], tri.points[2], tri.points[0]],
                2 => [tri.points[2], tri.points[0], tri.points[1]],
                _ => unreachable!()
            };
        }
    }
    
    /// Write out this STL in textual format.
    pub fn write_text_stl<I: io::Write>(&self, writer: &mut I) -> io::Result<()>
    {
        writeln!(writer, "solid {}", self.name)?;

        for tri in &self.triangles {
            let StlTriangle { normal, points } = tri;

            writeln!(writer, "facet normal {} {} {}", normal.x, normal.y, normal.z)?;
            writeln!(writer, "  outer loop")?;
            for point in points {
                writeln!(writer, "    vertex {} {} {}", point.x, point.y, point.z)?;
            }
            writeln!(writer, "  endloop")?;
            writeln!(writer, "endfacet")?;

        }

        writeln!(writer, "endsolid {}", self.name)?;

        Ok(())
    }
}
