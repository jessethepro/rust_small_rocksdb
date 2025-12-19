# RocksDB Build Automation

This repository includes automated workflows for building size-optimized RocksDB static libraries.

## Quick Start

### Trigger Automated Build (GitHub Actions)

1. **Standard Size-Optimized Build**
   - Go to Actions → "Build RocksDB Library"
   - Click "Run workflow"
   - Optional: Specify RocksDB version (default: v10.9.0)

2. **Advanced Custom Build**
   - Go to Actions → "Build RocksDB Library (Advanced)"
   - Configure options:
     - RocksDB version
     - Optimization level (`Os`, `O3`, `O2`, `Oz`)
     - Enable/disable LTO
     - Enable/disable compression libraries

### Local Build

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install -y build-essential cmake \
  libsnappy-dev zlib1g-dev libbz2-dev liblz4-dev libzstd-dev

# Clone RocksDB
git clone --depth 1 --branch v10.9.0 https://github.com/facebook/rocksdb.git

# Build with size optimizations
cd rocksdb
export EXTRA_CXXFLAGS="-Os -ffunction-sections -fdata-sections -fno-exceptions -fno-rtti -DNDEBUG"
export EXTRA_LDFLAGS="-Wl,--gc-sections -Wl,--strip-all"

make static_lib -j$(nproc) \
  DEBUG_LEVEL=0 \
  USE_RTTI=0 \
  DISABLE_WARNING_AS_ERROR=1 \
  PORTABLE=1 \
  EXTRA_CXXFLAGS="$EXTRA_CXXFLAGS" \
  EXTRA_LDFLAGS="$EXTRA_LDFLAGS"

strip --strip-unneeded librocksdb.a

# Copy to project
cp librocksdb.a ../lib/
```

## Build Configurations

### Size Optimization Flags

| Flag | Purpose | Size Impact |
|------|---------|-------------|
| `-Os` | Optimize for size | High |
| `-ffunction-sections` | Separate functions for linker GC | Medium |
| `-fdata-sections` | Separate data for linker GC | Medium |
| `-fno-exceptions` | Disable C++ exceptions | High |
| `-fno-rtti` | Disable runtime type info | Medium |
| `-Wl,--gc-sections` | Remove unused sections | High |
| `-Wl,--strip-all` | Strip all symbols | Medium |
| `strip --strip-unneeded` | Additional stripping | Low |

### Make Variables

```bash
DEBUG_LEVEL=0          # No debug symbols
USE_RTTI=0             # Disable RTTI
PORTABLE=1             # Portable binary (no CPU-specific optimizations)
DISABLE_WARNING_AS_ERROR=1  # Don't fail on warnings
```

### Compression Libraries

By default, RocksDB includes multiple compression libraries. Disable them for smaller size:

```bash
make static_lib \
  DISABLE_SNAPPY=1 \
  DISABLE_ZLIB=1 \
  DISABLE_BZIP=1 \
  DISABLE_LZ4=1 \
  DISABLE_ZSTD=1 \
  ...
```

**Trade-off**: Loses compression support, but reduces binary by ~30%.

## Workflow Features

### Standard Workflow (`build-rocksdb.yml`)
- Automatic builds on push/PR
- Caches RocksDB source
- Size-optimized flags pre-configured
- Uploads artifacts (30 day retention)
- Runs Rust tests to verify library
- Creates releases on tags

### Advanced Workflow (`build-rocksdb-advanced.yml`)
- Manual trigger only
- Configurable optimization levels
- Optional LTO (Link Time Optimization)
- Optional compression library exclusion
- Library analysis and size comparison
- 90-day artifact retention

## Size Benchmarks

Typical sizes for RocksDB v10.9.0:

| Configuration | Size | Notes |
|---------------|------|-------|
| Default build | ~50MB | Debug symbols, RTTI, exceptions |
| `-Os` optimized | ~13-15MB | No debug, stripped |
| `-Os` + no compression | ~10-12MB | Compression libs removed |
| `-Oz` + LTO | ~8-10MB | Maximum optimization |

## Artifacts

Downloaded artifacts include:
```
librocksdb-v10.9.0-size-optimized/
├── lib/
│   ├── librocksdb.a       # Static library
│   └── build-info.txt     # Build metadata
└── include/
    └── rocksdb/           # Header files
        ├── c.h
        ├── db.h
        └── ...
```

## Using Built Library

1. **Download from GitHub Actions**:
   - Go to Actions → Successful workflow run
   - Download artifact
   - Extract to your project

2. **Replace existing library**:
   ```bash
   cp librocksdb.a /path/to/rust_small_rocksdb/lib/
   ```

3. **Build your project**:
   ```bash
   cargo build --release
   ```

## Troubleshooting

### Build fails with "undefined reference"

Ensure C++ stdlib is linked in `build.rs`:
```rust
println!("cargo:rustc-link-lib=stdc++");  // Linux
// or
println!("cargo:rustc-link-lib=c++");     // macOS
```

### Library too large

Try advanced workflow with:
- Optimization: `Oz` (aggressive size)
- LTO: Enabled
- Compression: Disabled

### Tests fail after library update

1. Check RocksDB version compatibility
2. Verify all required symbols are present:
   ```bash
   nm lib/librocksdb.a | grep rocksdb_open
   ```
3. Rebuild Rust project from clean:
   ```bash
   cargo clean && cargo build
   ```

## Advanced: Custom Build Script

See `scripts/build-rocksdb.sh` for a reusable local build script.

## References

- [RocksDB Build Documentation](https://github.com/facebook/rocksdb/blob/main/INSTALL.md)
- [GCC Optimization Options](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [Linker Garbage Collection](https://interrupt.memfault.com/blog/best-firmware-size-tools#gc-sections)
