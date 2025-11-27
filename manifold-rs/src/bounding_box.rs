use crate::{Vec3, raw};

pub struct BoundingBox {
    pub(crate) ptr: *mut raw::ManifoldBox,
}

impl BoundingBox {
    /// Take ownership of an already-allocated box pointer.
    /// 
    /// Safety: The pointer must be valid, unique, and point to an allocated box instance.
    /// The instance must be initialised, if not already, before further use.
    unsafe fn from_raw(ptr: *mut raw::ManifoldBox) -> Self {
        Self { ptr }
    }

    /// Allocate a zero-sized box.
    /// 
    /// Safety: This is actually safe, as the FFI interface allocates a valid empty box.
    /// However, further initialisation is probably intended. If you really want an empty box,
    /// use [`BoundingBox::new`].
    unsafe fn alloc() -> Self {
        unsafe {
            Self::from_raw(raw::manifold_alloc_box())
        }
    }

    /// Create an empty, zero-sized box.
    pub fn new() -> Self {
        unsafe { Self::alloc() }
    }

    /// The lower point of the two which define the box.
    pub fn min_point(&self) -> Vec3 {
        unsafe {
            raw::manifold_box_min(self.ptr).into()
        }
    }

    /// The upper point of the two which define the box.
    pub fn max_point(&self) -> Vec3 {
        unsafe {
            raw::manifold_box_max(self.ptr).into()
        }
    }

    /// The size (dimensions) of the box.
    pub fn size(&self) -> Vec3 {
        unsafe {
            raw::manifold_box_dimensions(self.ptr).into()
        }
    }
}

impl Drop for BoundingBox {
    fn drop(&mut self) {
        unsafe {
            raw::manifold_delete_box(self.ptr);
        }
    }
}
