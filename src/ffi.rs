//! Raw FFI bindings to RocksDB C API
//!
//! This module contains unsafe bindings to the RocksDB C library.
//! These are low-level and should not be used directly - use the safe
//! wrappers in the parent module instead.

use libc::{c_char, c_int, c_void, size_t};

// Opaque types from RocksDB C API
#[repr(C)]
pub struct rocksdb_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_options_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_readoptions_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_writeoptions_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_iterator_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rocksdb_column_family_handle_t {
    _private: [u8; 0],
}

// Compile-time assertions to ensure opaque types are zero-sized
// This verifies that the types are truly opaque and don't accidentally grow
const _: () = {
    const fn assert_zero_sized<T>() {
        assert!(std::mem::size_of::<T>() == 0);
    }

    assert_zero_sized::<rocksdb_t>();
    assert_zero_sized::<rocksdb_options_t>();
    assert_zero_sized::<rocksdb_readoptions_t>();
    assert_zero_sized::<rocksdb_writeoptions_t>();
    assert_zero_sized::<rocksdb_iterator_t>();
    assert_zero_sized::<rocksdb_column_family_handle_t>();
};

// External functions from RocksDB C API
unsafe extern "C" {
    // Database operations
    pub fn rocksdb_open(
        options: *const rocksdb_options_t,
        name: *const c_char,
        errptr: *mut *mut c_char,
    ) -> *mut rocksdb_t;

    pub fn rocksdb_open_for_read_only(
        options: *const rocksdb_options_t,
        name: *const c_char,
        error_if_wal_file_exists: c_int,
        errptr: *mut *mut c_char,
    ) -> *mut rocksdb_t;

    pub fn rocksdb_close(db: *mut rocksdb_t);

    pub fn rocksdb_put(
        db: *mut rocksdb_t,
        options: *const rocksdb_writeoptions_t,
        key: *const c_char,
        keylen: size_t,
        val: *const c_char,
        vallen: size_t,
        errptr: *mut *mut c_char,
    );

    pub fn rocksdb_get(
        db: *mut rocksdb_t,
        options: *const rocksdb_readoptions_t,
        key: *const c_char,
        keylen: size_t,
        vallen: *mut size_t,
        errptr: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn rocksdb_delete(
        db: *mut rocksdb_t,
        options: *const rocksdb_writeoptions_t,
        key: *const c_char,
        keylen: size_t,
        errptr: *mut *mut c_char,
    );

    // Options
    pub fn rocksdb_options_create() -> *mut rocksdb_options_t;
    pub fn rocksdb_options_destroy(options: *mut rocksdb_options_t);
    pub fn rocksdb_options_set_create_if_missing(options: *mut rocksdb_options_t, value: c_int);
    pub fn rocksdb_options_set_error_if_exists(options: *mut rocksdb_options_t, value: c_int);

    // Read options
    pub fn rocksdb_readoptions_create() -> *mut rocksdb_readoptions_t;
    pub fn rocksdb_readoptions_destroy(options: *mut rocksdb_readoptions_t);

    // Write options
    pub fn rocksdb_writeoptions_create() -> *mut rocksdb_writeoptions_t;
    pub fn rocksdb_writeoptions_destroy(options: *mut rocksdb_writeoptions_t);
    pub fn rocksdb_writeoptions_set_sync(options: *mut rocksdb_writeoptions_t, value: c_int);

    // Iterator operations
    pub fn rocksdb_create_iterator(
        db: *mut rocksdb_t,
        options: *const rocksdb_readoptions_t,
    ) -> *mut rocksdb_iterator_t;

    pub fn rocksdb_iter_destroy(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_valid(iter: *const rocksdb_iterator_t) -> u8;
    pub fn rocksdb_iter_seek_to_first(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_seek_to_last(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_seek(iter: *mut rocksdb_iterator_t, key: *const c_char, klen: size_t);
    pub fn rocksdb_iter_seek_for_prev(
        iter: *mut rocksdb_iterator_t,
        key: *const c_char,
        klen: size_t,
    );
    pub fn rocksdb_iter_next(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_prev(iter: *mut rocksdb_iterator_t);
    pub fn rocksdb_iter_key(iter: *const rocksdb_iterator_t, klen: *mut size_t) -> *const c_char;
    pub fn rocksdb_iter_value(iter: *const rocksdb_iterator_t, vlen: *mut size_t) -> *const c_char;
    pub fn rocksdb_iter_get_error(iter: *const rocksdb_iterator_t, errptr: *mut *mut c_char);

    // Memory management
    pub fn rocksdb_free(ptr: *mut c_void);

    // Column family operations
    pub fn rocksdb_create_column_family(
        db: *mut rocksdb_t,
        column_family_options: *const rocksdb_options_t,
        column_family_name: *const c_char,
        errptr: *mut *mut c_char,
    ) -> *mut rocksdb_column_family_handle_t;

    pub fn rocksdb_drop_column_family(
        db: *mut rocksdb_t,
        handle: *mut rocksdb_column_family_handle_t,
        errptr: *mut *mut c_char,
    );

    pub fn rocksdb_column_family_handle_destroy(handle: *mut rocksdb_column_family_handle_t);

    // Column family read/write operations
    pub fn rocksdb_put_cf(
        db: *mut rocksdb_t,
        options: *const rocksdb_writeoptions_t,
        column_family: *mut rocksdb_column_family_handle_t,
        key: *const c_char,
        keylen: size_t,
        val: *const c_char,
        vallen: size_t,
        errptr: *mut *mut c_char,
    );

    pub fn rocksdb_get_cf(
        db: *mut rocksdb_t,
        options: *const rocksdb_readoptions_t,
        column_family: *mut rocksdb_column_family_handle_t,
        key: *const c_char,
        keylen: size_t,
        vallen: *mut size_t,
        errptr: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn rocksdb_delete_cf(
        db: *mut rocksdb_t,
        options: *const rocksdb_writeoptions_t,
        column_family: *mut rocksdb_column_family_handle_t,
        key: *const c_char,
        keylen: size_t,
        errptr: *mut *mut c_char,
    );

    // Open database with column families
    pub fn rocksdb_open_column_families(
        options: *const rocksdb_options_t,
        name: *const c_char,
        num_column_families: c_int,
        column_family_names: *const *const c_char,
        column_family_options: *const *const rocksdb_options_t,
        column_family_handles: *mut *mut rocksdb_column_family_handle_t,
        errptr: *mut *mut c_char,
    ) -> *mut rocksdb_t;

    pub fn rocksdb_list_column_families(
        options: *const rocksdb_options_t,
        name: *const c_char,
        lencf: *mut size_t,
        errptr: *mut *mut c_char,
    ) -> *mut *mut c_char;
}
