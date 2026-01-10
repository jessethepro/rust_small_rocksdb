# Copilot Instructions: rust_small_rocksdb

## Project Overview
Rust bindings for RocksDB - provides safe, idiomatic Rust wrappers around RocksDB C API. The codebase includes RocksDB 10.9.0 headers and a prebuilt static library (librocksdb.a). This is a minimal, focused binding library designed for simplicity and safety over feature completeness.

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
- **Size-optimized library** - Custom RocksDB build with `-Os`/`-Oz`, LTO, and section GC reduces binary size by ~30-50% vs standard builds

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
cargo build --release  # Release build with optimizations
```

**Build process** ([build.rs](build.rs)):
1. Locates `lib/librocksdb.a` via `CARGO_MANIFEST_DIR`
2. Links static library: `cargo:rustc-link-lib=static=rocksdb`
3. Links C++ stdlib: `-lstdc++` (Linux) or `-lc++` (macOS)
4. Reruns if lib changes

### Rebuilding RocksDB Library
The project includes automation for building size-optimized RocksDB libraries. See [BUILD_AUTOMATION.md](BUILD_AUTOMATION.md) for details.

**GitHub Actions** (Recommended):
- Standard: `.github/workflows/build-rocksdb.yml` - Quick default build
- Advanced: `.github/workflows/build-rocksdb-advanced.yml` - Custom configuration

**Local build**:
```bash
./scripts/build-rocksdb.sh          # Default: v10.7.5 with -Os
./scripts/build-rocksdb.sh v10.9.0 Oz  # Custom version & optimization
```

**Size optimization flags** (from [scripts/build-rocksdb.sh](scripts/build-rocksdb.sh)):
- `-Os` or `-Oz`: Size optimization
- `-ffunction-sections -fdata-sections`: Section-level GC
- `-Wl,--gc-sections`: Link-time dead code removal
- `USE_RTTI=0`: Disable C++ RTTI for smaller binaries
- Optional: Disable compression libs (snappy, zlib) for 30% size reduction

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

**Why**: Tests use real filesystem operations with hardcoded `/tmp/` paths. No tempfile crate - manual cleanup required. Tests are NOT parallelizable due to potential path conflicts.

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

### 1. C Pointer Null Checks - The Double Check Pattern
RocksDB's C API uses an **error-out parameter pattern** that requires checking TWO things:

```rust
// WRONG: Only checks return pointer
let db_ptr = ffi::rocksdb_open(...);
if db_ptr.is_null() {  // Misses the error message!
    return Err(Error::new("Failed"));
}

// WRONG: Uses unchecked conversion
let db_ptr = ffi::rocksdb_open(...);
NonNull::new_unchecked(db_ptr)  // Segfault if open failed!

// CORRECT: Check error pointer FIRST, then return pointer
let mut err: *mut i8 = ptr::null_mut();
let db_ptr = ffi::rocksdb_open(opts, c_path.as_ptr(), &mut err);

if !err.is_null() {
    // Error occurred - extract message and free C string
    return Err(Error::from_c_string(err));
}

if db_ptr.is_null() {
    // Should never happen if errptr was null, but be defensive
    return Err(Error::new("Failed to open database"));
}
```

**Why this matters**: If you only check `db_ptr`, you miss the actual error message from RocksDB. If you don't check `err` at all, you leak the error string AND potentially dereference a null pointer.

### 2. Memory Leak - Forgetting to Free Error Strings
Every error string returned by RocksDB **MUST** be freed with `rocksdb_free()`:

```rust
// WRONG: Leaks memory on every error
let mut err: *mut i8 = ptr::null_mut();
ffi::rocksdb_put(db, ..., &mut err);
if !err.is_null() {
    let msg = CStr::from_ptr(err).to_string_lossy();
    return Err(Error::new(msg));  // LEAK! Never freed err
}

// CORRECT: Always free error strings (see src/error.rs)
unsafe fn from_c_string(ptr: *mut c_char) -> Self {
    let c_str = CStr::from_ptr(ptr);
    let message = c_str.to_string_lossy().into_owned();
    rocksdb_free(ptr as *mut c_void);  // Critical!
    Error { message }
}
```

**Memory leak sources**:
- Error strings from all FFI operations (`put`, `get`, `delete`, `open`)
- Values from `rocksdb_get()` (must use `rocksdb_free`, not Rust's allocator)
- Iterator error checking with `rocksdb_iter_get_error()`

### 3. Path String Conversions with Embedded Nulls
CString creation can fail if the path contains null bytes:

```rust
// WRONG: Panics on path with null byte
let path = "/tmp/my\0db";  // Embedded null
let c_path = CString::new(path).unwrap();  // PANIC!

// CORRECT: Handle conversion errors (see src/db.rs)
let c_path = CString::new(path.to_string_lossy().as_bytes())
    .map_err(|_| Error::new("Invalid path"))?;
```

**Edge case**: On Unix, paths can contain any bytes except null. Windows paths have more restrictions. Always validate path conversion.

### 4. Binary Data Handling - Null Bytes are Valid
RocksDB is binary-safe. Keys and values are `&[u8]`, **NOT** strings:

```rust
// WRONG: Assumes UTF-8 keys
let key = "user:123";
db.put(key.as_bytes(), value)?;
let retrieved = String::from_utf8(db.get(key.as_bytes())?.unwrap())?;  // Can crash!

// CORRECT: Always work with byte slices
let key = b"user:123";  // or arbitrary bytes
db.put(key, value)?;
let value_bytes = db.get(key)?;  // Always returns Vec<u8>

// Valid keys that would break string assumptions:
let binary_key = b"\x00\x01\xff\xfe";  // Null bytes, invalid UTF-8
db.put(binary_key, b"value")?;  // Perfectly valid!
```

**Why this matters**: 
- Use case: Composite keys with binary-packed integers (`[prefix][u64][u32]`)
- Use case: Storing serialized protobufs as keys
- Never use `str::from_utf8_unchecked()` on RocksDB data

### 5. Iterator Lifetimes - The Borrow Checker is Your Friend
Iterators hold a **borrowed reference** to the DB with lifetime `'a`:

```rust
// WRONG: DB dropped while iterator still exists
fn get_first_key(path: &str) -> Option<Vec<u8>> {
    let db = DB::open(&opts, path).unwrap();
    let mut iter = db.raw_iterator();  // Borrows &db
    iter.seek_to_first();
    
    drop(db);  // ERROR: Can't drop DB while iter exists!
    
    iter.key().map(|k| k.to_vec())  // Use-after-free!
}

// CORRECT: Ensure DB outlives iterator
fn get_first_key(path: &str) -> Option<Vec<u8>> {
    let db = DB::open(&opts, path).unwrap();
    let mut iter = db.raw_iterator();
    iter.seek_to_first();
    
    let key = iter.key().map(|k| k.to_vec());
    drop(iter);  // Drop iterator first
    drop(db);    // Then DB
    key
}

// BEST: Let Rust's scope rules handle it
fn get_first_key(path: &str) -> Option<Vec<u8>> {
    let db = DB::open(&opts, path).unwrap();
    let mut iter = db.raw_iterator();
    iter.seek_to_first();
    iter.key().map(|k| k.to_vec())  // Both dropped at end of scope
}
```

**Compiler will catch this**, but understanding the lifetime relationship prevents confusion when refactoring.

### 6. Options Lifetime During DB::open
Options must remain valid **during** the `DB::open` call, but RocksDB copies them internally:

```rust
// WRONG: Options dropped too early (won't compile)
let db = {
    let opts = Options::default();
    DB::open(&opts, path)?  // opts dropped before open completes
};

// CORRECT: Options can be dropped after open
let mut opts = Options::default();
opts.create_if_missing(true);
let db = DB::open(&opts, path)?;
drop(opts);  // Safe! DB has its own copy

// ALSO CORRECT: Temporary options
let db = DB::open(&Options::default(), path)?;  // Drops after call
```

### 7. Read/Write Options Are Not Cached
Every `get()`, `put()`, `delete()` creates new read/write options internally. For high-throughput code, consider exposing reusable options:

```rust
// Current pattern (creates options on each call):
for i in 0..1000000 {
    db.put(format!("key{}", i).as_bytes(), b"value")?;
    // Allocates and destroys rocksdb_writeoptions_t each time
}

// Future optimization: Reusable write options
let write_opts = WriteOptions::new();
write_opts.set_sync(false);
for i in 0..1000000 {
    db.put_with_opts(&write_opts, ...)?;  // Reuse options
}
```

**Note**: This is a future extension point mentioned in the docs.

### 8. Test Path Conflicts
Tests use hardcoded `/tmp/` paths that can conflict if run in parallel:

```rust
// WRONG: Running tests in parallel
// Terminal 1: cargo test test_put_and_get
// Terminal 2: cargo test test_delete
// Both write to /tmp/rust_rocksdb_test_* → race conditions!

// CORRECT: Run tests serially
cargo test -- --test-threads=1

// OR: Use unique paths per test (current approach)
// Each test uses a different path suffix:
// test_put_and_get     → /tmp/rust_rocksdb_test_put_get
// test_delete          → /tmp/rust_rocksdb_test_delete
```

**Why no tempfile crate?** Deliberate choice to minimize dependencies. Trade-off: Manual cleanup required.

### 9. Platform-Specific Linker Errors
Missing C++ standard library is the #1 build issue on new systems:

```bash
# WRONG: Generic error message doesn't help
error: undefined reference to `std::__cxx11::basic_string`

# Diagnosis:
nm lib/librocksdb.a | grep "std::"  # Check if C++ symbols present
ldd target/debug/librust_small_rocksdb.so  # Check linked libraries

# Fix for Linux:
# Verify build.rs has: println!("cargo:rustc-link-lib=stdc++");

# Fix for macOS:
# Verify build.rs has: println!("cargo:rustc-link-lib=c++");
```

### 10. Forgetting to Check Iterator Errors
Iterators can fail internally. Always check for errors after iteration:

```rust
// WRONG: Silently ignores I/O errors during iteration
let mut iter = db.raw_iterator();
iter.seek_to_first();
while iter.valid() {
    process(iter.key(), iter.value());
    iter.next();
}
// What if disk I/O failed during iteration?

// CORRECT: Check iterator status (future extension)
let mut iter = db.raw_iterator();
iter.seek_to_first();
while iter.valid() {
    process(iter.key(), iter.value());
    iter.next();
}
// Future: iter.status()? to check for errors
```

**Current limitation**: This crate doesn't yet expose `rocksdb_iter_get_error()`. When adding it, follow the error string freeing pattern from pitfall #2.

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
- Rust edition: 2024
- Crate types: `staticlib`, `rlib` (see [Cargo.toml](Cargo.toml))

## Debugging and Troubleshooting

### Common Build Issues

**Linker errors about undefined C++ symbols**:
- Linux: Verify `-lstdc++` is linked (check [build.rs](build.rs))
- macOS: Verify `-lc++` is linked
- Missing librocksdb.a: Run `./scripts/build-rocksdb.sh` or use GitHub Actions

**Test failures**:
- `/tmp/` not writable: Tests require write access to `/tmp/`
- Path conflicts: Don't run tests in parallel with `cargo test -- --test-threads=1`
- DB lock issues: Ensure previous test cleaned up (check for orphaned `/tmp/rust_rocksdb_test_*` directories)

**FFI crashes**:
- Always check `errptr` for null before using returned pointers
- Verify all C strings are properly freed with `rocksdb_free`
- Use `NonNull::new()` not `new_unchecked()` for C pointer validation

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
