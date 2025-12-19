# Copilot Instructions: rust_small_rocksdb

## Project Overview
Rust bindings for RocksDB - provides safe, idiomatic Rust wrappers around RocksDB C API. The codebase includes RocksDB 10.9.0 headers and a prebuilt static library (librocksdb.a).

## Architecture

### Directory Structure
```
include/rocksdb/    # 118 RocksDB C++ header files (upstream API)
lib/librocksdb.a    # Prebuilt static library (13MB)
src/
  ├── lib.rs        # Public API and re-exports
  ├── ffi.rs        # Raw FFI bindings (unsafe C API declarations)
  ├── db.rs         # Safe DB wrapper (337 lines)
  ├── options.rs    # Configuration builders
  ├── iterator.rs   # Iterator wrappers (213 lines)
  └── error.rs      # Error types with C string handling
tests/
  └── integration_tests.rs  # Full integration test suite (262 lines)
build.rs            # Links librocksdb.a and C++ stdlib
```

### Key Architectural Decisions
- **Rust 2024 edition** - Uses latest language features
- **Static linking** - Links librocksdb.a at compile time via [build.rs](build.rs)
- **Zero-copy design** - Returns borrowed slices where possible (iterators)
- **RAII safety** - All FFI types wrapped with Drop implementations
- **Thread-safe by default** - DB implements Send+Sync (RocksDB is thread-safe)

## FFI Safety Patterns

### Memory Management Rules
All wrapper types follow this pattern from [src/db.rs](src/db.rs):
```rust
pub struct DB {
    inner: NonNull<ffi::rocksdb_t>,  // Never null, validated at creation
    path: String,                     // Owns path string
}

impl Drop for DB {
    fn drop(&mut self) {
        unsafe { ffi::rocksdb_close(self.inner.as_ptr()) }
    }
}
```

**Critical**: Every RocksDB C type must have a matching `_destroy` or `_close` function called in Drop:
- `rocksdb_t` → `rocksdb_close`
- `rocksdb_options_t` → `rocksdb_options_destroy`
- `rocksdb_iterator_t` → `rocksdb_iter_destroy`
- C strings from RocksDB → `rocksdb_free`

### Error Handling Pattern
See [src/error.rs](src/error.rs) - errors are C strings that MUST be freed:
```rust
unsafe fn from_c_string(ptr: *mut c_char) -> Self {
    let c_str = CStr::from_ptr(ptr);
    let message = c_str.to_string_lossy().into_owned();
    rocksdb_free(ptr as *mut c_void);  // Always free!
    Error { message }
}
```

### Thread Safety
- `DB` and `Options` implement `Send` (safe to transfer between threads)
- `DB` implements `Sync` (safe to share references) - RocksDB DB handle is thread-safe
- Iterators borrow `DB` with lifetime `'a` - not Send/Sync by design

## Critical Workflows

### Building and Testing
```bash
cargo build          # Links librocksdb.a via build.rs
cargo test           # Runs integration tests in tests/
cargo clippy         # Linting
```

**Build process** ([build.rs](build.rs)):
1. Locates `lib/librocksdb.a` via `CARGO_MANIFEST_DIR`
2. Links static library: `cargo:rustc-link-lib=static=rocksdb`
3. Links C++ stdlib: `-lstdc++` (Linux) or `-lc++` (macOS)
4. Reruns if lib changes

### Test Pattern
All tests in [tests/integration_tests.rs](tests/integration_tests.rs) follow this structure:
```rust
let path = "/tmp/rust_rocksdb_test_*";
let _ = fs::remove_dir_all(path);  // Clean before test
let mut opts = Options::default();
opts.create_if_missing(true);
let db = DB::open(&opts, path)?;
// ... test operations ...
drop(db);
let _ = fs::remove_dir_all(path);  // Clean after test
```

**Why**: Tests use real filesystem operations with hardcoded `/tmp/` paths. No tempfile crate - manual cleanup required.

## API Usage Patterns

### Basic CRUD (from [src/db.rs](src/db.rs))
```rust
let db = DB::open(&opts, path)?;
db.put(b"key", b"value")?;           // Returns Result<()>
let val = db.get(b"key")?;           // Returns Result<Option<Vec<u8>>>
db.delete(b"key")?;
```

### Iterator Pattern (from [src/iterator.rs](src/iterator.rs))
Two iterator APIs:
1. **High-level** (Rust Iterator trait):
   ```rust
   for item in db.iter(Direction::Forward) {
       let (key, value) = item?;  // Borrows from iterator
   }
   ```

2. **Low-level** (manual control):
   ```rust
   let mut iter = db.raw_iterator();
   iter.seek(b"start_key");
   while iter.valid() {
       let key = iter.key();    // Returns Option<&[u8]>
       let value = iter.value();
       iter.next();
   }
   ```

### Options Builder Pattern
```rust
let mut opts = Options::default();
opts.create_if_missing(true)
    .error_if_exists(false);
```

**Note**: Options uses fluent API (returns `&mut self`). See [src/options.rs](src/options.rs).

## Common Pitfalls

### 1. C Pointer Null Checks
```rust
// WRONG: Assumes pointer is valid
let db_ptr = ffi::rocksdb_open(...);
NonNull::new_unchecked(db_ptr)  // Crashes if null!

// CORRECT: Check error pointer first
let mut err: *mut i8 = ptr::null_mut();
let db_ptr = ffi::rocksdb_open(opts, path, &mut err);
if !err.is_null() {
    return Err(Error::from_c_string(err));
}
```

### 2. Binary Data Handling
RocksDB keys/values can contain null bytes. Always use byte slices, never assume UTF-8:
```rust
let key = b"\x00\x01\xff";  // Valid RocksDB key
db.put(key, value)?;
```

### 3. Iterator Lifetimes
Iterators borrow the DB - ensure DB outlives iterator:
```rust
let iter = db.raw_iterator();
drop(db);  // ERROR: DB dropped while iter still exists
iter.key(); // Use-after-free!
```

### 4. C++ Standard Library
If you get linker errors about undefined C++ symbols:
- Linux: Needs `-lstdc++` (handled in [build.rs](build.rs))
- macOS: Needs `-lc++` (handled in [build.rs](build.rs))

## Integration Points

### FFI Bindings ([src/ffi.rs](src/ffi.rs))
All bindings are manual declarations of C API from `include/rocksdb/c.h`:
```rust
extern "C" {
    pub fn rocksdb_open(
        options: *const rocksdb_options_t,
        name: *const c_char,
        errptr: *mut *mut c_char,  // Error out-parameter
    ) -> *mut rocksdb_t;
}
```

**Pattern**: Most functions take `errptr: *mut *mut c_char` for error reporting. Always check and free this pointer.

### Opaque Types
All RocksDB types are opaque (zero-sized):
```rust
#[repr(C)]
pub struct rocksdb_t {
    _private: [u8; 0],  // Cannot construct, only via FFI
}
```

### Version Info
- RocksDB: 10.9.0 (from `include/rocksdb/version.h`)
- Library type: Static archive (`.a` file)

## Future Extension Points

### Adding New Options
1. Declare FFI function in [src/ffi.rs](src/ffi.rs)
2. Add method to `Options` in [src/options.rs](src/options.rs) following fluent builder pattern
3. Test in [tests/integration_tests.rs](tests/integration_tests.rs)

### Adding New DB Operations
Follow pattern in [src/db.rs](src/db.rs):
1. Wrap unsafe FFI call with error handling
2. Convert between Rust types (`&[u8]`) and C types (`*const c_char, size_t`)
3. Return `Result<T>` for fallible operations
4. Document with example

## Documentation Standards
- All `unsafe` blocks must document safety invariants (see [src/error.rs](src/error.rs#L23))
- Public API examples must be `no_run` (require filesystem)
- Module-level docs explain "why", not just "what" (see [src/ffi.rs](src/ffi.rs#L1-5))
- Link to upstream docs: https://github.com/facebook/rocksdb/wiki
