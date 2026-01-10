//! Rust bindings for RocksDB
//!
//! This crate provides safe Rust wrappers around the RocksDB C API.
//!
//! # Example
//!
//! ```no_run
//! use rust_small_rocksdb::{DB, Options};
//!
//! let mut opts = Options::default();
//! opts.create_if_missing(true);
//!
//! let db = DB::open(&opts, "/tmp/test_db").unwrap();
//! db.put(b"key", b"value").unwrap();
//!
//! let value = db.get(b"key").unwrap();
//! assert_eq!(value.as_deref(), Some(&b"value"[..]));
//!
//! db.delete(b"key").unwrap();
//! ```

mod db;
mod error;
mod ffi;
mod iterator;
mod options;

pub use db::{ColumnFamilyHandle, DB};
pub use error::{Error, Result};
pub use iterator::{DBIterator, DBIteratorAdapter, Direction};
pub use options::Options;
