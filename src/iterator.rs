//! Iterator for traversing RocksDB key-value pairs

use crate::error::{Error, Result};
use crate::ffi;
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::slice;

/// Iterator direction
pub enum Direction {
    /// Iterate forward from the current position
    Forward,
    /// Iterate backward from the current position
    Reverse,
}

/// An iterator over the key-value pairs in a RocksDB database
///
/// This iterator provides a way to traverse the database in sorted key order.
/// The iterator borrows the database and read options for its lifetime.
pub struct DBIterator<'a> {
    inner: NonNull<ffi::rocksdb_iterator_t>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> DBIterator<'a> {
    /// Create a new iterator (internal use only)
    pub(crate) unsafe fn new(inner: NonNull<ffi::rocksdb_iterator_t>) -> Self {
        DBIterator {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Check if the iterator is positioned at a valid entry
    pub fn valid(&self) -> bool {
        unsafe { ffi::rocksdb_iter_valid(self.inner.as_ptr()) != 0 }
    }

    /// Position the iterator at the first key in the database
    pub fn seek_to_first(&mut self) {
        unsafe {
            ffi::rocksdb_iter_seek_to_first(self.inner.as_ptr());
        }
    }

    /// Position the iterator at the last key in the database
    pub fn seek_to_last(&mut self) {
        unsafe {
            ffi::rocksdb_iter_seek_to_last(self.inner.as_ptr());
        }
    }

    /// Position the iterator at the first key greater than or equal to the target
    pub fn seek<K: AsRef<[u8]>>(&mut self, key: K) {
        let key = key.as_ref();
        unsafe {
            ffi::rocksdb_iter_seek(self.inner.as_ptr(), key.as_ptr() as *const i8, key.len());
        }
    }

    /// Position the iterator at the first key less than or equal to the target
    pub fn seek_for_prev<K: AsRef<[u8]>>(&mut self, key: K) {
        let key = key.as_ref();
        unsafe {
            ffi::rocksdb_iter_seek_for_prev(
                self.inner.as_ptr(),
                key.as_ptr() as *const i8,
                key.len(),
            );
        }
    }

    /// Move to the next entry
    pub fn next(&mut self) {
        unsafe {
            ffi::rocksdb_iter_next(self.inner.as_ptr());
        }
    }

    /// Move to the previous entry
    pub fn prev(&mut self) {
        unsafe {
            ffi::rocksdb_iter_prev(self.inner.as_ptr());
        }
    }

    /// Get the key at the current position
    ///
    /// Returns None if the iterator is not positioned at a valid entry
    pub fn key(&self) -> Option<&[u8]> {
        if !self.valid() {
            return None;
        }

        unsafe {
            let mut klen: usize = 0;
            let key_ptr = ffi::rocksdb_iter_key(self.inner.as_ptr(), &mut klen);
            if key_ptr.is_null() {
                None
            } else {
                Some(slice::from_raw_parts(key_ptr as *const u8, klen))
            }
        }
    }

    /// Get the value at the current position
    ///
    /// Returns None if the iterator is not positioned at a valid entry
    pub fn value(&self) -> Option<&[u8]> {
        if !self.valid() {
            return None;
        }

        unsafe {
            let mut vlen: usize = 0;
            let value_ptr = ffi::rocksdb_iter_value(self.inner.as_ptr(), &mut vlen);
            if value_ptr.is_null() {
                None
            } else {
                Some(slice::from_raw_parts(value_ptr as *const u8, vlen))
            }
        }
    }

    /// Get both key and value at the current position
    ///
    /// Returns None if the iterator is not positioned at a valid entry
    pub fn item(&self) -> Option<(&[u8], &[u8])> {
        if !self.valid() {
            return None;
        }
        Some((self.key()?, self.value()?))
    }

    /// Check for any error that occurred during iteration
    pub fn status(&self) -> Result<()> {
        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            ffi::rocksdb_iter_get_error(self.inner.as_ptr(), &mut err);

            if err.is_null() {
                Ok(())
            } else {
                Err(Error::from_c_string(err))
            }
        }
    }
}

impl<'a> Drop for DBIterator<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_iter_destroy(self.inner.as_ptr());
        }
    }
}

/// Iterator adapter that yields Result<(Box<[u8]>, Box<[u8]>)>
///
/// This is useful for iterating over the database in a Rust-idiomatic way
/// using a for loop.
pub struct DBIteratorAdapter<'a> {
    inner: DBIterator<'a>,
    direction: Direction,
    just_seeked: bool,
}

impl<'a> DBIteratorAdapter<'a> {
    /// Create a new iterator adapter
    pub(crate) fn new(inner: DBIterator<'a>, direction: Direction) -> Self {
        DBIteratorAdapter {
            inner,
            direction,
            just_seeked: true, // Iterator is already positioned at first/last
        }
    }
}

impl<'a> Iterator for DBIteratorAdapter<'a> {
    type Item = Result<(Box<[u8]>, Box<[u8]>)>;

    fn next(&mut self) -> Option<Self::Item> {
        // Move to next position if we're not at the initial seek position
        if !self.just_seeked {
            match self.direction {
                Direction::Forward => self.inner.next(),
                Direction::Reverse => self.inner.prev(),
            }
        }
        self.just_seeked = false;

        // Check if iterator is valid
        if !self.inner.valid() {
            // Check for errors
            return match self.inner.status() {
                Ok(()) => None,
                Err(e) => Some(Err(e)),
            };
        }

        // Get key and value
        match self.inner.item() {
            Some((key, value)) => {
                let key = key.to_vec().into_boxed_slice();
                let value = value.to_vec().into_boxed_slice();
                Some(Ok((key, value)))
            }
            None => None,
        }
    }
}
