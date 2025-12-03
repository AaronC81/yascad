use std::ops;

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

impl<T: ops::Add<Output = T>> ops::Add for Vec3<T> {
    type Output = Vec3<T>;

    fn add(self, rhs: Self) -> Self::Output {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<T: ops::Sub<Output = T>> ops::Sub for Vec3<T> {
    type Output = Vec3<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
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


#[derive(Clone, Copy, Debug, PartialEq, Default, PartialOrd)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

impl<T> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Default> Vec2<T> {
    pub fn zero() -> Self {
        Self::default()
    }
}

impl<T: ops::Add<Output = T>> ops::Add for Vec2<T> {
    type Output = Vec2<T>;

    fn add(self, rhs: Self) -> Self::Output {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T: ops::Sub<Output = T>> ops::Sub for Vec2<T> {
    type Output = Vec2<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl From<raw::ManifoldVec2> for Vec2<f64> {
    fn from(value: raw::ManifoldVec2) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

