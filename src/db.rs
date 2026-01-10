//! RocksDB database handle

use crate::error::{Error, Result};
use crate::ffi;
use crate::iterator;
use crate::options::Options;
use std::ffi::CString;
use std::path::Path;
use std::ptr::{self, NonNull};

/// RAII guard for RocksDB write options
///
/// Automatically destroys the write options when dropped, ensuring
/// no resource leaks even if an error occurs.
struct WriteOptionsGuard(*mut ffi::rocksdb_writeoptions_t);

impl WriteOptionsGuard {
    /// Create new write options
    fn new() -> Result<Self> {
        unsafe {
            let ptr = ffi::rocksdb_writeoptions_create();
            if ptr.is_null() {
                Err(Error::new("Failed to create write options"))
            } else {
                Ok(WriteOptionsGuard(ptr))
            }
        }
    }

    /// Get the raw pointer for FFI calls
    fn as_ptr(&self) -> *mut ffi::rocksdb_writeoptions_t {
        self.0
    }
}

impl Drop for WriteOptionsGuard {
    fn drop(&mut self) {
        // Catch panics to prevent double-panic during unwinding
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            ffi::rocksdb_writeoptions_destroy(self.0);
        }));
    }
}

/// RAII guard for RocksDB read options
///
/// Automatically destroys the read options when dropped, ensuring
/// no resource leaks even if an error occurs.
struct ReadOptionsGuard(*mut ffi::rocksdb_readoptions_t);

impl ReadOptionsGuard {
    /// Create new read options
    fn new() -> Result<Self> {
        unsafe {
            let ptr = ffi::rocksdb_readoptions_create();
            if ptr.is_null() {
                Err(Error::new("Failed to create read options"))
            } else {
                Ok(ReadOptionsGuard(ptr))
            }
        }
    }

    /// Get the raw pointer for FFI calls
    fn as_ptr(&self) -> *mut ffi::rocksdb_readoptions_t {
        self.0
    }
}

impl Drop for ReadOptionsGuard {
    fn drop(&mut self) {
        // Catch panics to prevent double-panic during unwinding
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            ffi::rocksdb_readoptions_destroy(self.0);
        }));
    }
}

/// RAII wrapper for byte arrays allocated by RocksDB
///
/// This ensures that memory returned by RocksDB (via `rocksdb_get`, etc.)
/// is properly freed using `rocksdb_free` instead of Rust's allocator.
/// Implements Deref to allow transparent access to the underlying slice.
struct OwnedRocksDBBytes {
    ptr: *mut u8,
    len: usize,
}

impl OwnedRocksDBBytes {
    /// Create from a raw pointer and length returned by RocksDB
    ///
    /// # Safety
    /// - ptr must be allocated by RocksDB or be null
    /// - if ptr is not null, it must point to at least len bytes
    /// - ptr must not be used after this call (ownership is transferred)
    unsafe fn from_raw(ptr: *mut i8, len: usize) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(OwnedRocksDBBytes {
                ptr: ptr as *mut u8,
                len,
            })
        }
    }

    /// Get a slice view of the data
    fn as_slice(&self) -> &[u8] {
        unsafe {
            // SAFETY: ptr is guaranteed valid for len bytes for the lifetime of Self
            std::slice::from_raw_parts(self.ptr, self.len)
        }
    }
}

impl Drop for OwnedRocksDBBytes {
    fn drop(&mut self) {
        // Catch panics to prevent double-panic during unwinding
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            // SAFETY: ptr was allocated by RocksDB and must be freed with rocksdb_free
            ffi::rocksdb_free(self.ptr as *mut std::ffi::c_void);
        }));
    }
}

impl std::ops::Deref for OwnedRocksDBBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for OwnedRocksDBBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

/// A RocksDB column family handle
///
/// Column families provide a way to logically partition data within a single database.
/// Each column family can have its own configuration and be managed independently.
#[must_use = "Column family handle must be stored or it will be immediately destroyed"]
pub struct ColumnFamilyHandle {
    inner: NonNull<ffi::rocksdb_column_family_handle_t>,
}

impl ColumnFamilyHandle {
    /// Get the raw pointer for FFI calls (internal use only)
    pub(crate) fn as_ptr(&self) -> *mut ffi::rocksdb_column_family_handle_t {
        self.inner.as_ptr()
    }
}

impl Drop for ColumnFamilyHandle {
    fn drop(&mut self) {
        // Catch panics to prevent double-panic during unwinding
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            ffi::rocksdb_column_family_handle_destroy(self.inner.as_ptr());
        }));
    }
}

// ColumnFamilyHandle is safe to send between threads
unsafe impl Send for ColumnFamilyHandle {}

/// A RocksDB database handle
///
/// This is the main interface for interacting with a RocksDB database.
/// The database is automatically closed when the DB instance is dropped.
#[must_use = "Database handle must be stored or the database will be immediately closed"]
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

            let inner =
                NonNull::new(db_ptr).ok_or_else(|| Error::new("Failed to open database"))?;

            Ok(DB {
                inner,
                path: path.to_string_lossy().into_owned(),
            })
        }
    }

    /// Open a RocksDB database with existing column families
    ///
    /// This opens a database that has column families and returns handles to all of them.
    /// The "default" column family is always present and will be the first handle returned.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the database
    /// * `path` - Path to the database directory
    /// * `cf_names` - Names of column families to open (include "default" for the default CF)
    /// * `cf_options` - Options for each column family (must match length of cf_names)
    ///
    /// # Returns
    ///
    /// A tuple of (DB, Vec<ColumnFamilyHandle>) where handles correspond to cf_names order
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rust_small_rocksdb::{DB, Options};
    ///
    /// let mut opts = Options::default();
    /// opts.create_if_missing(true);
    ///
    /// // Open with default and custom column families
    /// let cf_names = vec!["default", "users", "posts"];
    /// let cf_opts = vec![Options::default(), Options::default(), Options::default()];
    ///
    /// let (db, cf_handles) = DB::open_with_column_families(
    ///     &opts,
    ///     "/tmp/my_db",
    ///     &cf_names,
    ///     &cf_opts
    /// ).unwrap();
    ///
    /// // cf_handles[0] is "default", cf_handles[1] is "users", cf_handles[2] is "posts"
    /// db.put_cf(&cf_handles[1], b"user:1", b"Alice").unwrap();
    /// ```
    pub fn open_with_column_families<P: AsRef<Path>>(
        options: &Options,
        path: P,
        cf_names: &[&str],
        cf_options: &[Options],
    ) -> Result<(Self, Vec<ColumnFamilyHandle>)> {
        if cf_names.len() != cf_options.len() {
            return Err(Error::new(
                "Number of column family names must match number of options",
            ));
        }

        if cf_names.is_empty() {
            return Err(Error::new("Must specify at least one column family"));
        }

        let path = path.as_ref();
        let c_path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| Error::new("Invalid path"))?;

        // Convert column family names to C strings
        let c_cf_names: Result<Vec<CString>> = cf_names
            .iter()
            .map(|name| CString::new(*name).map_err(|_| Error::new("Invalid column family name")))
            .collect();
        let c_cf_names = c_cf_names?;

        // Create array of pointers to C strings
        let cf_name_ptrs: Vec<*const i8> = c_cf_names.iter().map(|s| s.as_ptr()).collect();

        // Create array of pointers to options
        let cf_option_ptrs: Vec<*const ffi::rocksdb_options_t> =
            cf_options.iter().map(|opt| opt.as_ptr()).collect();

        // Allocate space for column family handles
        let mut cf_handle_ptrs: Vec<*mut ffi::rocksdb_column_family_handle_t> =
            vec![ptr::null_mut(); cf_names.len()];

        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            let db_ptr = ffi::rocksdb_open_column_families(
                options.as_ptr(),
                c_path.as_ptr(),
                cf_names.len() as i32,
                cf_name_ptrs.as_ptr(),
                cf_option_ptrs.as_ptr(),
                cf_handle_ptrs.as_mut_ptr(),
                &mut err,
            );

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            let inner =
                NonNull::new(db_ptr).ok_or_else(|| Error::new("Failed to open database"))?;

            // Convert raw pointers to ColumnFamilyHandle
            let cf_handles: Result<Vec<ColumnFamilyHandle>> = cf_handle_ptrs
                .into_iter()
                .map(|ptr| {
                    NonNull::new(ptr)
                        .map(|inner| ColumnFamilyHandle { inner })
                        .ok_or_else(|| Error::new("Failed to get column family handle"))
                })
                .collect();

            Ok((
                DB {
                    inner,
                    path: path.to_string_lossy().into_owned(),
                },
                cf_handles?,
            ))
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

            let inner = NonNull::new(db_ptr)
                .ok_or_else(|| Error::new("Failed to open database in read-only mode"))?;

            Ok(DB {
                inner,
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
        // Debug assertions: validate that slices are properly formed
        debug_assert!(
            key.len() < isize::MAX as usize,
            "Key length exceeds maximum safe size"
        );
        debug_assert!(
            value.len() < isize::MAX as usize,
            "Value length exceeds maximum safe size"
        );

        let write_opts = WriteOptionsGuard::new()?;

        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            ffi::rocksdb_put(
                self.inner.as_ptr(),
                write_opts.as_ptr(),
                key.as_ptr() as *const i8,
                key.len(),
                value.as_ptr() as *const i8,
                value.len(),
                &mut err,
            );

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
        // Debug assertion: validate that key slice is properly formed
        debug_assert!(
            key.len() < isize::MAX as usize,
            "Key length exceeds maximum safe size"
        );

        let read_opts = ReadOptionsGuard::new()?;

        unsafe {
            let mut val_len: usize = 0;
            let mut err: *mut i8 = ptr::null_mut();
            let val_ptr = ffi::rocksdb_get(
                self.inner.as_ptr(),
                read_opts.as_ptr(),
                key.as_ptr() as *const i8,
                key.len(),
                &mut val_len,
                &mut err,
            );

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            // Use OwnedRocksDBBytes to safely manage RocksDB-allocated memory
            Ok(OwnedRocksDBBytes::from_raw(val_ptr, val_len).map(|bytes| bytes.to_vec()))
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
        // Debug assertion: validate that key slice is properly formed
        debug_assert!(
            key.len() < isize::MAX as usize,
            "Key length exceeds maximum safe size"
        );

        let write_opts = WriteOptionsGuard::new()?;

        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            ffi::rocksdb_delete(
                self.inner.as_ptr(),
                write_opts.as_ptr(),
                key.as_ptr() as *const i8,
                key.len(),
                &mut err,
            );

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
            let read_opts = ReadOptionsGuard::new().expect("Failed to create read options");
            let iter_ptr = ffi::rocksdb_create_iterator(self.inner.as_ptr(), read_opts.as_ptr());

            // read_opts is automatically destroyed here

            let iter_non_null = NonNull::new(iter_ptr).expect("Failed to create iterator");
            let mut db_iter = DBIterator::new(iter_non_null);

            // Position iterator based on direction
            match direction {
                iterator::Direction::Forward => db_iter.seek_to_first(),
                iterator::Direction::Reverse => db_iter.seek_to_last(),
            }

            DBIteratorAdapter::new(db_iter, direction)
        }
    }

    /// Create a new column family with the given options
    ///
    /// Column families allow you to logically partition your data within a single database.
    /// Each column family can have its own configuration and be managed independently.
    ///
    /// # Arguments
    ///
    /// * `options` - Configuration options for the new column family
    /// * `name` - Name of the column family to create
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
    /// // Create a column family for user data
    /// let cf_opts = Options::default();
    /// let cf_handle = db.create_column_family(&cf_opts, "users").unwrap();
    /// ```
    pub fn create_column_family(
        &self,
        options: &Options,
        name: &str,
    ) -> Result<ColumnFamilyHandle> {
        let c_name = CString::new(name).map_err(|_| Error::new("Invalid column family name"))?;

        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            let cf_handle = ffi::rocksdb_create_column_family(
                self.inner.as_ptr(),
                options.as_ptr(),
                c_name.as_ptr(),
                &mut err,
            );

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            let inner = NonNull::new(cf_handle)
                .ok_or_else(|| Error::new("Failed to create column family"))?;

            Ok(ColumnFamilyHandle { inner })
        }
    }

    /// Drop (delete) a column family
    ///
    /// This permanently removes the column family and all of its data.
    /// The column family handle becomes invalid after this call.
    ///
    /// # Arguments
    ///
    /// * `cf_handle` - Handle to the column family to drop
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
    /// let cf_opts = Options::default();
    /// let cf_handle = db.create_column_family(&cf_opts, "temp").unwrap();
    ///
    /// // Drop the column family when no longer needed
    /// db.drop_column_family(cf_handle).unwrap();
    /// ```
    pub fn drop_column_family(&self, cf_handle: ColumnFamilyHandle) -> Result<()> {
        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            ffi::rocksdb_drop_column_family(self.inner.as_ptr(), cf_handle.as_ptr(), &mut err);

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            Ok(())
        }
    }

    /// Put a key-value pair into a specific column family
    ///
    /// # Arguments
    ///
    /// * `cf_handle` - Handle to the column family
    /// * `key` - The key to store
    /// * `value` - The value to store
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
    /// let cf_opts = Options::default();
    /// let cf_handle = db.create_column_family(&cf_opts, "users").unwrap();
    ///
    /// db.put_cf(&cf_handle, b"user:1", b"Alice").unwrap();
    /// ```
    pub fn put_cf(&self, cf_handle: &ColumnFamilyHandle, key: &[u8], value: &[u8]) -> Result<()> {
        debug_assert!(
            key.len() < isize::MAX as usize,
            "Key length exceeds maximum safe size"
        );
        debug_assert!(
            value.len() < isize::MAX as usize,
            "Value length exceeds maximum safe size"
        );

        let write_opts = WriteOptionsGuard::new()?;

        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            ffi::rocksdb_put_cf(
                self.inner.as_ptr(),
                write_opts.as_ptr(),
                cf_handle.as_ptr(),
                key.as_ptr() as *const i8,
                key.len(),
                value.as_ptr() as *const i8,
                value.len(),
                &mut err,
            );

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            Ok(())
        }
    }

    /// Get a value from a specific column family
    ///
    /// Returns `None` if the key doesn't exist in the column family.
    ///
    /// # Arguments
    ///
    /// * `cf_handle` - Handle to the column family
    /// * `key` - The key to retrieve
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
    /// let cf_opts = Options::default();
    /// let cf_handle = db.create_column_family(&cf_opts, "users").unwrap();
    ///
    /// db.put_cf(&cf_handle, b"user:1", b"Alice").unwrap();
    /// let value = db.get_cf(&cf_handle, b"user:1").unwrap();
    /// assert_eq!(value.as_deref(), Some(&b"Alice"[..]));
    /// ```
    pub fn get_cf(&self, cf_handle: &ColumnFamilyHandle, key: &[u8]) -> Result<Option<Vec<u8>>> {
        debug_assert!(
            key.len() < isize::MAX as usize,
            "Key length exceeds maximum safe size"
        );

        let read_opts = ReadOptionsGuard::new()?;

        unsafe {
            let mut val_len: usize = 0;
            let mut err: *mut i8 = ptr::null_mut();
            let val_ptr = ffi::rocksdb_get_cf(
                self.inner.as_ptr(),
                read_opts.as_ptr(),
                cf_handle.as_ptr(),
                key.as_ptr() as *const i8,
                key.len(),
                &mut val_len,
                &mut err,
            );

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            Ok(OwnedRocksDBBytes::from_raw(val_ptr, val_len).map(|bytes| bytes.to_vec()))
        }
    }

    /// Delete a key from a specific column family
    ///
    /// # Arguments
    ///
    /// * `cf_handle` - Handle to the column family
    /// * `key` - The key to delete
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
    /// let cf_opts = Options::default();
    /// let cf_handle = db.create_column_family(&cf_opts, "users").unwrap();
    ///
    /// db.put_cf(&cf_handle, b"user:1", b"Alice").unwrap();
    /// db.delete_cf(&cf_handle, b"user:1").unwrap();
    /// assert_eq!(db.get_cf(&cf_handle, b"user:1").unwrap(), None);
    /// ```
    pub fn delete_cf(&self, cf_handle: &ColumnFamilyHandle, key: &[u8]) -> Result<()> {
        debug_assert!(
            key.len() < isize::MAX as usize,
            "Key length exceeds maximum safe size"
        );

        let write_opts = WriteOptionsGuard::new()?;

        unsafe {
            let mut err: *mut i8 = ptr::null_mut();
            ffi::rocksdb_delete_cf(
                self.inner.as_ptr(),
                write_opts.as_ptr(),
                cf_handle.as_ptr(),
                key.as_ptr() as *const i8,
                key.len(),
                &mut err,
            );

            if !err.is_null() {
                return Err(Error::from_c_string(err));
            }

            Ok(())
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
            let read_opts = ReadOptionsGuard::new().expect("Failed to create read options");
            let iter_ptr = ffi::rocksdb_create_iterator(self.inner.as_ptr(), read_opts.as_ptr());
            // read_opts is automatically destroyed here

            let iter_non_null = NonNull::new(iter_ptr).expect("Failed to create iterator");
            DBIterator::new(iter_non_null)
        }
    }
}

impl Drop for DB {
    fn drop(&mut self) {
        // Catch panics to prevent double-panic during unwinding
        // SAFETY: self.inner is always valid during the lifetime of DB
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            ffi::rocksdb_close(self.inner.as_ptr());
        }));
    }
}

// DB is safe to send between threads (RocksDB DB handle is thread-safe)
unsafe impl Send for DB {}
// DB is safe to share between threads (RocksDB DB handle is thread-safe)
unsafe impl Sync for DB {}
