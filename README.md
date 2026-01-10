# rust_small_rocksdb

[![Crates.io](https://img.shields.io/crates/v/rust_small_rocksdb.svg)](https://crates.io/crates/rust_small_rocksdb)
[![Documentation](https://docs.rs/rust_small_rocksdb/badge.svg)](https://docs.rs/rust_small_rocksdb)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Minimal, safe Rust bindings for RocksDB with column family support.

## Features

- ✅ **Simple API** - Easy-to-use interface for basic RocksDB operations
- ✅ **Safe** - Memory-safe wrappers with RAII, panic-safe Drop, and compile-time checks
- ✅ **Column Families** - Full support for creating, reading, writing, and dropping column families
- ✅ **Zero Dependencies** - Only depends on `libc` (RocksDB is statically linked)
- ✅ **Size Optimized** - Custom RocksDB build with `-Os`/`-Oz` and LTO for minimal binary size
- ✅ **Thread Safe** - DB handle is Send + Sync (RocksDB is thread-safe)
- ✅ **Rust 2024** - Uses latest Rust edition

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust_small_rocksdb = "0.1"
```

### Basic Usage

```rust
use rust_small_rocksdb::{DB, Options};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open database
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, "/tmp/my_db")?;

    // Put and get
    db.put(b"key", b"value")?;
    let value = db.get(b"key")?;
    assert_eq!(value.as_deref(), Some(&b"value"[..]));

    // Delete
    db.delete(b"key")?;

    Ok(())
}
```

### Column Families

```rust
use rust_small_rocksdb::{DB, Options};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, "/tmp/my_db")?;

    // Create column family
    let cf_opts = Options::default();
    let users_cf = db.create_column_family(&cf_opts, "users")?;

    // Write to column family
    db.put_cf(&users_cf, b"user:1", b"Alice")?;
    
    // Read from column family
    let value = db.get_cf(&users_cf, b"user:1")?;
    assert_eq!(value.as_deref(), Some(&b"Alice"[..]));

    // Delete from column family
    db.delete_cf(&users_cf, b"user:1")?;

    Ok(())
}
```

### Iterators

```rust
use rust_small_rocksdb::{DB, Options, Direction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    let db = DB::open(&opts, "/tmp/my_db")?;

    // Insert data
    db.put(b"key1", b"value1")?;
    db.put(b"key2", b"value2")?;

    // Iterate forward
    for item in db.iter(Direction::Forward) {
        let (key, value) = item?;
        println!("{:?} => {:?}", key, value);
    }

    Ok(())
}
```

## API Reference

### Core Types

- **`DB`** - Main database handle with thread-safe operations
- **`Options`** - Configuration for database and column families
- **`ColumnFamilyHandle`** - Handle to a column family
- **`DBIterator`** - Low-level iterator with manual control
- **`DBIteratorAdapter`** - High-level iterator implementing Rust's `Iterator` trait
- **`Direction`** - Iterator direction (Forward/Reverse)
- **`Error`** - Error type for all operations

### Database Operations

```rust
// Opening
DB::open(&opts, path) -> Result<DB>
DB::open_for_read_only(&opts, path, error_if_wal) -> Result<DB>
DB::open_with_column_families(&opts, path, cf_names, cf_opts) -> Result<(DB, Vec<ColumnFamilyHandle>)>

// Basic operations
db.put(key, value) -> Result<()>
db.get(key) -> Result<Option<Vec<u8>>>
db.delete(key) -> Result<()>

// Column family operations
db.create_column_family(&opts, name) -> Result<ColumnFamilyHandle>
db.drop_column_family(handle) -> Result<()>
db.put_cf(&handle, key, value) -> Result<()>
db.get_cf(&handle, key) -> Result<Option<Vec<u8>>>
db.delete_cf(&handle, key) -> Result<()>

// Iteration
db.iter(direction) -> DBIteratorAdapter
db.raw_iterator() -> DBIterator

// Properties
db.path() -> &str
```

## Column Families

Column families provide logical data partitioning within a single database:

- **Isolation**: Same key can exist in different CFs with different values
- **Configuration**: Each CF can have independent settings
- **Efficiency**: Atomic writes across multiple CFs
- **Deletion**: Drop entire CF quickly without scanning keys

See [COLUMN_FAMILIES.md](COLUMN_FAMILIES.md) for detailed documentation.

## Examples

```bash
# Basic usage
cargo run --example basic

# Column families demo
cargo run --example column_family_demo
```

## Building

The project includes a prebuilt RocksDB static library (`lib/librocksdb.a`). To rebuild:

```bash
# Default build (v10.7.5 with -Os)
./scripts/build-rocksdb.sh

# Custom version and optimization
./scripts/build-rocksdb.sh v10.9.0 Oz
```

Or use GitHub Actions for cross-platform builds. See [BUILD_AUTOMATION.md](BUILD_AUTOMATION.md).

## Testing

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test integration_tests
cargo test --test column_family_tests

# Run with output
cargo test -- --nocapture
```

## Documentation

```bash
# Generate and open documentation
cargo doc --open
```

## Safety

This crate follows strict safety practices:

- **RAII**: All resources automatically cleaned up on drop
- **NonNull**: Safe pointer construction, no `new_unchecked()`
- **Panic Safety**: All Drop implementations catch panics
- **Debug Assertions**: Validate invariants in debug builds
- **Compile-time Checks**: Zero-sized type assertions for FFI types
- **Memory Management**: Proper use of `rocksdb_free()` for RocksDB-allocated memory

## Architecture

- **Static Linking**: Links `librocksdb.a` at compile time
- **Zero-Copy**: Returns borrowed slices where possible
- **Thread-Safe**: DB implements Send + Sync
- **Size-Optimized**: Custom build with `-Os`/`-Oz`, LTO, and section GC

See [.github/copilot-instructions.md](.github/copilot-instructions.md) for detailed architecture documentation.

## RocksDB Version

This crate includes RocksDB 10.9.0 headers and a prebuilt static library.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! This is a minimal binding library focused on simplicity and safety. When contributing:

1. Maintain the simple API surface
2. Follow existing safety patterns (RAII, NonNull, panic-safe Drop)
3. Add tests for new features
4. Update documentation

## Acknowledgments

- [RocksDB](https://github.com/facebook/rocksdb) - The underlying key-value store
- Built with Rust 2024 edition

## See Also

- [RocksDB Wiki](https://github.com/facebook/rocksdb/wiki)
- [Column Families Documentation](COLUMN_FAMILIES.md)
- [Build Automation](BUILD_AUTOMATION.md)
