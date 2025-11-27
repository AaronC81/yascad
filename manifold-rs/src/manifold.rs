use std::os::raw::c_void;

use crate::{BoundingBox, meshgl::MeshGL, raw};

pub struct Manifold {
    pub(crate) ptr: *mut raw::ManifoldManifold,
}

impl Manifold {
    /// Take ownership of an already-allocated manifold pointer.
    /// 
    /// Safety: The pointer must be valid, unique, and point to an allocated manifold instance.
    /// The instance must be initialised, if not already, before further use.
    unsafe fn from_raw(ptr: *mut raw::ManifoldManifold) -> Self {
        Self { ptr }
    }

    /// Allocate an empty manifold.
    /// 
    /// Safety: This is actually safe, as the FFI interface allocates a valid empty manifold.
    /// However, further initialisation is probably intended. If you really want an empty manifold,
    /// use [`Manifold::new`].
    unsafe fn alloc() -> Self {
        unsafe {
            Self::from_raw(raw::manifold_alloc_manifold())
        }
    }

    /// Shorthand for `alloc` followed by an initialiser on the raw pointer.
    /// 
    /// Safety: See `alloc`.
    unsafe fn alloc_build<R>(func: impl FnOnce(*mut c_void) -> R) -> Self {
        unsafe {
            let manifold = Self::alloc();
            func(manifold.ptr as *mut c_void);
            manifold
        }
    }

    /// Create an empty manifold.
    pub fn new() -> Self {
        unsafe { Self::alloc() }
    }

    /// Create a manifold of a cube.
    pub fn cube(x: f64, y: f64, z: f64, centre: bool) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_cube(ptr, x, y, z, if centre { 1 } else { 0 }))
        }
    }

    /// Create a new manifold which is a translation of this one.
    pub fn translate(&self, x: f64, y: f64, z: f64) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_translate(ptr, self.ptr, x, y, z))
        }
    }

    /// Create a new manifold which is a rotation of this one.
    pub fn rotate(&self, x: f64, y: f64, z: f64) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_rotate(ptr, self.ptr, x, y, z))
        }
    }

    /// Create a new manifold by subtracting another manifold from this one.
    pub fn difference(&self, other: &Manifold) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_difference(ptr, self.ptr, other.ptr))
        }
    }

    /// Create a new manifold which combines two others.
    pub fn union(&self, other: &Manifold) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_union(ptr, self.ptr, other.ptr))
        }
    }

    /// Get a [`MeshGL`] for this manifold.
    pub fn meshgl(&self) -> MeshGL {
        MeshGL::from_manifold(self)
    }

    pub fn bounding_box(&self) -> BoundingBox {
        unsafe {
            let bbox = BoundingBox::new();
            raw::manifold_bounding_box(bbox.ptr as *mut c_void, self.ptr);
            bbox
        }
    }
}

impl Clone for Manifold {
    fn clone(&self) -> Self {
        unsafe {
            Self::alloc_build(|ptr|
                raw::manifold_copy(ptr, self.ptr))
        }
    }
}

impl Drop for Manifold {
    fn drop(&mut self) {
        unsafe {
            raw::manifold_delete_manifold(self.ptr);
        }
    }
}
