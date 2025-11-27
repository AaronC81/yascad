use crate::manifold::Manifold;

mod raw;
pub mod manifold;
pub mod meshgl;

fn main() {
    println!("Hello, world!");

    let cube = Manifold::cube(1.0, 1.0, 10.0, false);
    cube.meshgl().export("rust.stl");

    println!("Done");
}

