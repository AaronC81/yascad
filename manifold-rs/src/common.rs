use crate::raw;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self::zero()
    }
}

impl From<raw::ManifoldVec3> for Vec3 {
    fn from(value: raw::ManifoldVec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}
