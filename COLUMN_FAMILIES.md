# Column Family Features

This document describes the column family support implemented in rust_small_rocksdb.

## Overview

Column families provide logical data partitioning within a single RocksDB database. Each column family can have its own configuration, and operations can be performed independently on each family.

## Features Implemented

### 1. Create Column Family
Create a new column family with custom options:

```rust
let cf_handle = db.create_column_family(&cf_opts, "users")?;
```

### 2. Column Family Operations

#### Put (Write)
Write key-value pairs to a specific column family:

```rust
db.put_cf(&cf_handle, b"user:1", b"Alice")?;
```

#### Get (Read)
Read values from a specific column family:

```rust
let value = db.get_cf(&cf_handle, b"user:1")?;
```

#### Delete
Delete keys from a specific column family:

```rust
db.delete_cf(&cf_handle, b"user:1")?;
```

### 3. Drop Column Family
Permanently delete a column family and all its data:

```rust
db.drop_column_family(cf_handle)?;
```

### 4. Open with Column Families
Open an existing database with known column families:

```rust
let cf_names = vec!["default", "users", "posts"];
let cf_opts = vec![Options::default(), Options::default(), Options::default()];

let (db, cf_handles) = DB::open_with_column_families(
    &opts,
    "/tmp/my_db",
    &cf_names,
    &cf_opts
)?;

// cf_handles[0] is "default"
// cf_handles[1] is "users"
// cf_handles[2] is "posts"
```

## Key Concepts

### Isolation
Column families are completely isolated from each other. The same key can exist in multiple column families with different values:

```rust
db.put_cf(&users_cf, b"key", b"user data")?;
db.put_cf(&posts_cf, b"key", b"post data")?;

// Different values for the same key
let user_val = db.get_cf(&users_cf, b"key")?; // "user data"
let post_val = db.get_cf(&posts_cf, b"key")?; // "post data"
```

### Default Column Family
Every RocksDB database has a "default" column family. When you use the regular `put()`, `get()`, and `delete()` methods, you're operating on the default column family.

### Column Family Handles
Column family handles must be kept alive while in use. They implement Drop to ensure proper cleanup:

```rust
let cf_handle = db.create_column_family(&cf_opts, "users")?;
// Use cf_handle...
drop(cf_handle); // Automatically cleaned up
```

## Benefits

1. **Logical Partitioning**: Organize related data into separate families
2. **Independent Configuration**: Each family can have different options
3. **Efficient Operations**: Atomic writes across families
4. **Quick Deletion**: Drop entire families instantly
5. **Isolation**: Same keys in different families don't conflict

## Example Use Cases

### Multi-Tenant Database
```rust
let tenant1_cf = db.create_column_family(&cf_opts, "tenant_1")?;
let tenant2_cf = db.create_column_family(&cf_opts, "tenant_2")?;
```

### Time-Series Data
```rust
let today_cf = db.create_column_family(&cf_opts, "2026-01-10")?;
let yesterday_cf = db.create_column_family(&cf_opts, "2026-01-09")?;
// Drop old data quickly
db.drop_column_family(yesterday_cf)?;
```

### Application Data Types
```rust
let users_cf = db.create_column_family(&cf_opts, "users")?;
let posts_cf = db.create_column_family(&cf_opts, "posts")?;
let comments_cf = db.create_column_family(&cf_opts, "comments")?;
```

## Testing

Comprehensive tests are available in `tests/column_family_tests.rs`:

```bash
cargo test --test column_family_tests
```

Test coverage includes:
- Creating single and multiple column families
- Put/get/delete operations on column families
- Column family isolation verification
- Dropping column families
- Opening databases with existing column families
- Error handling for invalid names and parameters

## Demo

Run the interactive demo to see all features:

```bash
cargo run --example column_family_demo
```

The demo shows:
1. Creating column families
2. Writing data to different families
3. Reading data from families
4. Isolation between families
5. Deleting data from families
6. Reopening with existing families
7. Dropping column families

## FFI Bindings

All column family operations are backed by RocksDB C API bindings in `src/ffi.rs`:

- `rocksdb_create_column_family` - Create new CF
- `rocksdb_drop_column_family` - Delete CF
- `rocksdb_put_cf` - Write to CF
- `rocksdb_get_cf` - Read from CF
- `rocksdb_delete_cf` - Delete from CF
- `rocksdb_open_column_families` - Open with CFs
- `rocksdb_column_family_handle_destroy` - Cleanup handle

## Safety

All column family operations follow the same safety patterns as the rest of the crate:

- **RAII**: Column family handles automatically cleaned up on drop
- **Error Handling**: All C error strings properly freed
- **NonNull Pointers**: Safe pointer construction with `NonNull::new()`
- **Panic Safety**: Drop implementations catch panics
- **Debug Assertions**: Validate input sizes in debug builds

## Performance Considerations

- Column families share the same write-ahead log (WAL)
- Atomic writes across multiple column families are efficient
- Creating/dropping column families is relatively lightweight
- Each column family has its own memtable and SST files

## Limitations

- Maximum number of column families limited by RocksDB (typically 10,000+)
- Column family names cannot contain null bytes
- Must know column family names to open with `open_with_column_families()`
- Cannot iterate across multiple column families simultaneously

## See Also

- [RocksDB Column Families Wiki](https://github.com/facebook/rocksdb/wiki/Column-Families)
- API Documentation: `cargo doc --open`
- Integration Tests: `tests/column_family_tests.rs`
- Example: `examples/column_family_demo.rs`
