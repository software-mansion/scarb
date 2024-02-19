/// This struct encapsulates a stable ABI representation of information required to construct a slice.
///
/// Please be aware that the memory management of values passing the FFI-barrier is tricky.
/// The memory must be freed on the same side of the barrier, where the allocation was made.
#[repr(C)]
#[derive(Debug)]
#[doc(hidden)]
pub struct StableSlice<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> StableSlice<T> {
    /// Create a new `StableSlice` from a Vector.
    /// Note that the vector will not be deallocated automatically.
    /// Please make sure to use `into_owned` afterward, to free the memory.
    pub fn new(mut x: Vec<T>) -> Self {
        x.shrink_to_fit();
        assert_eq!(x.len(), x.capacity());
        let ptr = x.as_mut_ptr();
        let len = x.len();
        std::mem::forget(x);
        Self { ptr, len }
    }

    /// Convert to owned vector.
    pub fn into_owned(self) -> Vec<T> {
        unsafe { Vec::from_raw_parts(self.ptr, self.len, self.len) }
    }

    /// Returns raw pointer and length.
    /// Can be used to construct a slice.
    /// No ownership is transferred.
    pub fn raw_parts(&self) -> (*mut T, usize) {
        (self.ptr, self.len)
    }
}
