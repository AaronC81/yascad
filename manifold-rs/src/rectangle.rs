use crate::{Vec2, raw};

pub struct Rectangle {
    pub(crate) ptr: *mut raw::ManifoldRect,
}

impl Rectangle {
    /// Take ownership of an already-allocated rectangle pointer.
    /// 
    /// Safety: The pointer must be valid, unique, and point to an allocated rectangle instance.
    /// The instance must be initialised, if not already, before further use.
    pub(crate) unsafe fn from_raw(ptr: *mut raw::ManifoldRect) -> Self {
        Self { ptr }
    }

    /// Allocate an empty rectangle.
    /// 
    /// Safety: The returned rectangle is not initialised, and must be initialised before further
    /// use.
    pub(crate) unsafe fn alloc() -> Self {
        unsafe {
            Self::from_raw(raw::manifold_alloc_rect())
        }
    }

    /// The lower point of the two which define the rectangle.
    pub fn min_point(&self) -> Vec2<f64> {
        unsafe {
            raw::manifold_rect_min(self.ptr).into()
        }
    }

    /// The upper point of the two which define the rectangle.
    pub fn max_point(&self) -> Vec2<f64> {
        unsafe {
            raw::manifold_rect_max(self.ptr).into()
        }
    }

    /// The size (dimensions) of the rectangle.
    pub fn size(&self) -> Vec2<f64> {
        unsafe {
            raw::manifold_rect_dimensions(self.ptr).into()
        }
    }
}

impl Drop for Rectangle {
    fn drop(&mut self) {
        unsafe {
            raw::manifold_delete_rect(self.ptr);
        }
    }
}
