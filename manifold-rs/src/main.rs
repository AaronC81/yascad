mod raw;

mod manifold;
pub use manifold::*;

mod meshgl;
pub use meshgl::*;

mod bounding_box;
pub use bounding_box::*;

mod common;
pub use common::*;

fn main() {
    println!("Hello, world!");

    let cube = Manifold::cube(1.0, 1.0, 10.0, false);

    let out = cube
        .union(
            &cube.clone()
                .translate(2.0, 2.0, 0.0)
                .rotate(90.0, 0.0, 0.0));

    println!("{:?}", out.bounding_box().size());
    out.meshgl().export("rust.stl");

    println!("Done");
}

