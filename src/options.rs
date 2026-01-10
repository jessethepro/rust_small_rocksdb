//! Options for configuring RocksDB

use crate::ffi;
use std::ptr::NonNull;

/// Options for opening a RocksDB database
#[must_use = "Options must be used to open a database"]
pub struct Options {
    inner: NonNull<ffi::rocksdb_options_t>,
}

impl Options {
    /// Create a new Options instance with default settings
    pub fn new() -> Self {
        unsafe {
            let ptr = ffi::rocksdb_options_create();
            Options {
                inner: NonNull::new(ptr).expect("Failed to create options"),
            }
        }
    }

    /// Set whether to create the database if it doesn't exist
    pub fn create_if_missing(&mut self, value: bool) -> &mut Self {
        unsafe {
            ffi::rocksdb_options_set_create_if_missing(self.inner.as_ptr(), value as i32);
        }
        self
    }

    /// Set whether to error if the database already exists
    pub fn error_if_exists(&mut self, value: bool) -> &mut Self {
        unsafe {
            ffi::rocksdb_options_set_error_if_exists(self.inner.as_ptr(), value as i32);
        }
        self
    }

    /// Get the raw pointer for FFI calls
    pub(crate) fn as_ptr(&self) -> *const ffi::rocksdb_options_t {
        self.inner.as_ptr()
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Options {
    fn drop(&mut self) {
        // Catch panics to prevent double-panic during unwinding
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            ffi::rocksdb_options_destroy(self.inner.as_ptr());
        }));
    }
}

// Options is safe to send between threads
unsafe impl Send for Options {}
