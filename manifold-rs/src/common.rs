use crate::raw;

#[derive(Clone, Copy, Debug, PartialEq, Default, PartialOrd)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vec3<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl<T: Default> Vec3<T> {
    pub fn zero() -> Self {
        Self::default()
    }
}

impl From<raw::ManifoldVec3> for Vec3<f64> {
    fn from(value: raw::ManifoldVec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}
