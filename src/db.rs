//! RocksDB database handle

use crate::error::{Error, Result};
use crate::ffi;
use crate::iterator;
use crate::options::Options;
use std::ffi::CString;
use std::path::Path;
use std::ptr::{self, NonNull};

/// A RocksDB database handle
///
/// This is the main interface for interacting with a RocksDB database.
/// The database is automatically closed when the DB instance is dropped.
pub struct DB {
    inner: NonNull<ffi::rocksdb_t>,
    path: String,
}

impl DB {
    /// Open a RocksDB database with the given options
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the database
    /// * `path` - Path to the database directory
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_small_rocksdb::{DB, Options};
    ///
    /// let mut opts = Options::default();
    /// opts.create_if_missing(true);
    /// let db = DB::open(&opts, "/tmp/my_db").unwrap();
    /// ```
    pub fn open<P: AsRef<Path>>(options: &Options, path: P) -> Result<Self> {
        let path = path.as_ref();
        let c_path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| Error::new("Invalid path"))?;

        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            let db_ptr = ffi::rocksdb_open(options.as_ptr(), c_path.as_ptr(), &mut err);

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            if db_ptr.is_null() {
                return Err(Error::new("Failed to open database"));
            }

            Ok(DB {
                inner: NonNull::new_unchecked(db_ptr),
                path: path.to_string_lossy().into_owned(),
            })
        }
    }

    /// Open a RocksDB database in read-only mode
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the database
    /// * `path` - Path to the database directory
    /// * `error_if_wal_file_exists` - If true, error if WAL files exist
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_small_rocksdb::{DB, Options};
    ///
    /// let opts = Options::default();
    /// let db = DB::open_for_read_only(&opts, "/tmp/my_db", false).unwrap();
    /// ```
    pub fn open_for_read_only<P: AsRef<Path>>(
        options: &Options,
        path: P,
        error_if_wal_file_exists: bool,
    ) -> Result<Self> {
        let path = path.as_ref();
        let c_path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| Error::new("Invalid path"))?;

        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            let db_ptr = ffi::rocksdb_open_for_read_only(
                options.as_ptr(),
                c_path.as_ptr(),
                error_if_wal_file_exists as i32,
                &mut err,
            );

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            if db_ptr.is_null() {
                return Err(Error::new("Failed to open database in read-only mode"));
            }

            Ok(DB {
                inner: NonNull::new_unchecked(db_ptr),
                path: path.to_string_lossy().into_owned(),
            })
        }
    }

    /// Put a key-value pair into the database
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rust_small_rocksdb::{DB, Options};
    /// # let mut opts = Options::default();
    /// # opts.create_if_missing(true);
    /// # let db = DB::open(&opts, "/tmp/test").unwrap();
    /// db.put(b"my_key", b"my_value").unwrap();
    /// ```
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        unsafe {
            let write_opts = ffi::rocksdb_writeoptions_create();
            if write_opts.is_null() {
                return Err(Error::new("Failed to create write options"));
            }

            let mut err: *mut i8 = ptr::null_mut();
            ffi::rocksdb_put(
                self.inner.as_ptr(),
                write_opts,
                key.as_ptr() as *const i8,
                key.len(),
                value.as_ptr() as *const i8,
                value.len(),
                &mut err,
            );

            ffi::rocksdb_writeoptions_destroy(write_opts);

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            Ok(())
        }
    }

    /// Get a value from the database by key
    ///
    /// Returns `None` if the key doesn't exist.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rust_small_rocksdb::{DB, Options};
    /// # let mut opts = Options::default();
    /// # opts.create_if_missing(true);
    /// # let db = DB::open(&opts, "/tmp/test").unwrap();
    /// # db.put(b"my_key", b"my_value").unwrap();
    /// let value = db.get(b"my_key").unwrap();
    /// assert_eq!(value.as_deref(), Some(&b"my_value"[..]));
    /// ```
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        unsafe {
            let read_opts = ffi::rocksdb_readoptions_create();
            if read_opts.is_null() {
                return Err(Error::new("Failed to create read options"));
            }

            let mut val_len: usize = 0;
            let mut err: *mut i8 = ptr::null_mut();
            let val_ptr = ffi::rocksdb_get(
                self.inner.as_ptr(),
                read_opts,
                key.as_ptr() as *const i8,
                key.len(),
                &mut val_len,
                &mut err,
            );

            ffi::rocksdb_readoptions_destroy(read_opts);

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            if val_ptr.is_null() {
                return Ok(None);
            }

            let value = std::slice::from_raw_parts(val_ptr as *const u8, val_len).to_vec();
            ffi::rocksdb_free(val_ptr as *mut std::ffi::c_void);

            Ok(Some(value))
        }
    }

    /// Delete a key from the database
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rust_small_rocksdb::{DB, Options};
    /// # let mut opts = Options::default();
    /// # opts.create_if_missing(true);
    /// # let db = DB::open(&opts, "/tmp/test").unwrap();
    /// # db.put(b"my_key", b"my_value").unwrap();
    /// db.delete(b"my_key").unwrap();
    /// assert_eq!(db.get(b"my_key").unwrap(), None);
    /// ```
    pub fn delete(&self, key: &[u8]) -> Result<()> {
        unsafe {
            let write_opts = ffi::rocksdb_writeoptions_create();
            if write_opts.is_null() {
                return Err(Error::new("Failed to create write options"));
            }

            let mut err: *mut i8 = ptr::null_mut();
            ffi::rocksdb_delete(
                self.inner.as_ptr(),
                write_opts,
                key.as_ptr() as *const i8,
                key.len(),
                &mut err,
            );

            ffi::rocksdb_writeoptions_destroy(write_opts);

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            Ok(())
        }
    }

    /// Get the path where this database is stored
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Create an iterator to traverse the database
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_small_rocksdb::{DB, Options, Direction};
    ///
    /// let mut opts = Options::default();
    /// opts.create_if_missing(true);
    /// let db = DB::open(&opts, "/tmp/my_db").unwrap();
    ///
    /// // Insert some data
    /// db.put(b"key1", b"value1").unwrap();
    /// db.put(b"key2", b"value2").unwrap();
    ///
    /// // Iterate forward
    /// for item in db.iter(Direction::Forward) {
    ///     let (key, value) = item.unwrap();
    ///     println!("Key: {:?}, Value: {:?}", key, value);
    /// }
    /// ```
    pub fn iter(&self, direction: iterator::Direction) -> iterator::DBIteratorAdapter<'_> {
        use iterator::{DBIterator, DBIteratorAdapter};

        unsafe {
            // Create read options and pass to iterator
            // RocksDB internally copies what it needs from read_opts, so we can destroy it
            let read_opts = ffi::rocksdb_readoptions_create();
            let iter_ptr = ffi::rocksdb_create_iterator(self.inner.as_ptr(), read_opts);

            // Check if iterator creation succeeded
            if iter_ptr.is_null() {
                ffi::rocksdb_readoptions_destroy(read_opts);
                panic!("Failed to create iterator");
            }

            ffi::rocksdb_readoptions_destroy(read_opts);

            let mut db_iter = DBIterator::new(NonNull::new_unchecked(iter_ptr));

            // Position iterator based on direction
            match direction {
                iterator::Direction::Forward => db_iter.seek_to_first(),
                iterator::Direction::Reverse => db_iter.seek_to_last(),
            }

            DBIteratorAdapter::new(db_iter, direction)
        }
    }

    /// Create a raw iterator with more control
    ///
    /// This returns a DBIterator that you can manually position and traverse.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_small_rocksdb::{DB, Options};
    ///
    /// let mut opts = Options::default();
    /// opts.create_if_missing(true);
    /// let db = DB::open(&opts, "/tmp/my_db").unwrap();
    ///
    /// let mut iter = db.raw_iterator();
    /// iter.seek(b"key");
    /// if iter.valid() {
    ///     println!("Found key: {:?}", iter.key());
    /// }
    /// ```
    pub fn raw_iterator(&self) -> iterator::DBIterator<'_> {
        use iterator::DBIterator;

        unsafe {
            let read_opts = ffi::rocksdb_readoptions_create();
            let iter_ptr = ffi::rocksdb_create_iterator(self.inner.as_ptr(), read_opts);
            ffi::rocksdb_readoptions_destroy(read_opts);

            DBIterator::new(NonNull::new_unchecked(iter_ptr))
        }
    }
}

impl Drop for DB {
    fn drop(&mut self) {
        unsafe {
            ffi::rocksdb_close(self.inner.as_ptr());
        }
    }
}

// DB is safe to send between threads (RocksDB DB handle is thread-safe)
unsafe impl Send for DB {}
// DB is safe to share between threads (RocksDB DB handle is thread-safe)
unsafe impl Sync for DB {}
