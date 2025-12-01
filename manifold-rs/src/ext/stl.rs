use std::{cmp::Ordering, io};

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

    /// Sort the model's triangles.
    /// 
    /// By default, triangles are emitted in order of addition.
    /// If the source of the triangles is unstable but you'd like a diffable STL, you can sort them.
    /// The order of triangles has no effect on the STL behaviour.
    pub fn sort(&mut self) {
        self.triangles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
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
