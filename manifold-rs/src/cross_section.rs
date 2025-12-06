use std::{fmt::Debug, os::raw::c_void};

use crate::{Polygons, Rectangle, raw};

pub struct CrossSection {
    pub(crate) ptr: *mut raw::ManifoldCrossSection,
}

impl CrossSection {
    /// Take ownership of an already-allocated cross section pointer.
    /// 
    /// Safety: The pointer must be valid, unique, and point to an allocated cross section instance.
    /// The instance must be initialised, if not already, before further use.
    unsafe fn from_raw(ptr: *mut raw::ManifoldCrossSection) -> Self {
        Self { ptr }
    }

    /// Allocate an empty manifold.
    /// 
    /// Safety: The returned cross section is not initialised, and must be initialised before
    /// further use.
    unsafe fn alloc() -> Self {
        unsafe {
            Self::from_raw(raw::manifold_alloc_cross_section())
        }
    }

    /// Shorthand for `alloc` followed by an initialiser on the raw pointer.
    /// 
    /// Safety: `func` must initialise the pointee cross section.
    unsafe fn alloc_build<R>(func: impl FnOnce(*mut c_void) -> R) -> Self {
        unsafe {
            let manifold = Self::alloc();
            func(manifold.ptr as *mut c_void);
            manifold
        }
    }

    /// Create an empty cross section.
    pub fn new() -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_empty(ptr))
        }
    }

    /// Create a cross section of a square.
    pub fn square(x: f64, y: f64, centre: bool) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_square(ptr, x, y, if centre { 1 } else { 0 }))
        }
    }

    /// Create a cross section of a circle.
    pub fn circle(radius: f64, segments: i32) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_circle(ptr, radius, segments))
        }
    }

    /// Create a new cross section which is a translation of this one.
    pub fn translate(&self, x: f64, y: f64) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_translate(ptr, self.ptr, x, y))
        }
    }

    /// Create a new cross section which is a rotation of this one.
    pub fn rotate(&self, angle: f64) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_rotate(ptr, self.ptr, angle))
        }
    }

    /// Create a new cross section by scaling this one.
    pub fn scale(&self, x: f64, y: f64) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_scale(ptr, self.ptr, x, y))
        }
    }

    /// Create a new cross section by mirroring this one. The given x/y describe a vector through
    /// the origin, which the cross section is mirrored "through".
    pub fn mirror(&self, x: f64, y: f64) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_mirror(ptr, self.ptr, x, y))
        }
    }

    /// Create a new cross section by subtracting another manifold from this one.
    pub fn difference(&self, other: &CrossSection) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_difference(ptr, self.ptr, other.ptr))
        }
    }

    /// Create a new cross section which combines two others.
    pub fn union(&self, other: &CrossSection) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_union(ptr, self.ptr, other.ptr))
        }
    }

    /// Get the polygons for this cross section. 
    pub fn polygons(&self) -> Polygons {
        unsafe {
            let poly = Polygons::alloc();
            raw::manifold_cross_section_to_polygons(poly.ptr as *mut c_void, self.ptr);
            poly
        }
    }

    /// Get the bounding rectangle for this cross section.
    pub fn bounding_rectangle(&self) -> Rectangle {
        unsafe {
            let rect = Rectangle::alloc();
            raw::manifold_cross_section_bounds(rect.ptr as *mut c_void, self.ptr);
            rect
        }
    }
}

impl Clone for CrossSection {
    fn clone(&self) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cross_section_copy(ptr, self.ptr))
        }
    }
}

impl Drop for CrossSection {
    fn drop(&mut self) {
        unsafe {
            raw::manifold_delete_cross_section(self.ptr);
        }
    }
}

impl Debug for CrossSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: more detailed
        write!(f, "CrossSection")
    }
}
