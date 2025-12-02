use crate::raw;

pub struct Polygons {
    pub(crate) ptr: *mut raw::ManifoldPolygons,
}

impl Polygons {
    /// Take ownership of an already-allocated polygons pointer.
    /// 
    /// Safety: The pointer must be valid, unique, and point to an allocated polygons instance.
    /// The instance must be initialised, if not already, before further use.
    pub(crate) unsafe fn from_raw(ptr: *mut raw::ManifoldPolygons) -> Self {
        Self { ptr }
    }

    /// Allocate an empty list of polygons.
    /// 
    /// Safety: The returned set of polygons is not initialised, and must be initialised before
    /// further use.
    pub(crate) unsafe fn alloc() -> Self {
        unsafe {
            Self::from_raw(raw::manifold_alloc_polygons())
        }
    }
}

impl Drop for Polygons {
    fn drop(&mut self) {
        unsafe {
            raw::manifold_delete_polygons(self.ptr);
        }
    }
}
