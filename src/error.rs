//! Error types for RocksDB operations

use std::error::Error as StdError;
use std::ffi::CStr;
use std::fmt;
use std::os::raw::c_char;

/// Result type alias for RocksDB operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for RocksDB operations
#[derive(Debug, Clone)]
pub struct Error {
    message: String,
}

impl Error {
    /// Create a new error from a C string pointer
    ///
    /// # Safety
    /// The pointer must be a valid null-terminated C string allocated by RocksDB.
    /// This function will free the pointer using rocksdb_free.
    pub(crate) unsafe fn from_c_string(ptr: *mut c_char) -> Self {
        if ptr.is_null() {
            return Error {
                message: "Unknown error".to_string(),
            };
        }

        let c_str = unsafe { CStr::from_ptr(ptr) };
        let message = c_str.to_string_lossy().into_owned();

        // Free the C string allocated by RocksDB
        unsafe { crate::ffi::rocksdb_free(ptr as *mut std::ffi::c_void) };

        Error { message }
    }

    /// Create a new error from a string
    pub fn new(message: impl Into<String>) -> Self {
        Error {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RocksDB error: {}", self.message)
    }
}

impl StdError for Error {}
