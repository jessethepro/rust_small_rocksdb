# Automated RocksDB Build System

This project includes automated GitHub Actions workflows for building size-optimized RocksDB libraries.

## 🚀 Quick Start

### GitHub Actions (Automated)

**Standard Build** (Recommended)
1. Go to [Actions](../../actions/workflows/build-rocksdb.yml)
2. Click "Run workflow" → "Run workflow"
3. Wait ~10 minutes
4. Download artifact from workflow run

**Advanced Build** (Custom Configuration)
1. Go to [Actions](../../actions/workflows/build-rocksdb-advanced.yml)
2. Click "Run workflow"
3. Configure options:
   - RocksDB version (e.g., `v10.9.0`)
   - Optimization level: `Os` (size), `O3` (speed), `Oz` (aggressive size)
   - Enable LTO for additional 10-15% size reduction
   - Disable compression libraries for 30% size reduction
4. Download artifact when complete

### Local Build

```bash
# Quick build with defaults
./scripts/build-rocksdb.sh

# Custom version and optimization
./scripts/build-rocksdb.sh v9.7.0 Oz

# Then build your Rust project
cargo build --release
```

## 📦 What You Get

Artifacts include:
- `lib/librocksdb.a` - Optimized static library
- `lib/build-info.txt` - Build configuration details
- `include/rocksdb/` - C API headers

## 📊 Size Comparison

| Build Type | Size | Compression | Use Case |
|-----------|------|-------------|----------|
| Standard `-Os` | ~13MB | ✅ All | Recommended default |
| `-Os` no compression | ~10MB | ❌ None | Minimal binary size |
| `-Oz` + LTO | ~8MB | ✅ All | Maximum optimization |
| `-O3` | ~18MB | ✅ All | Performance focus |

## 🔧 Size Optimization Techniques

The workflows use aggressive size reduction:
- **`-Os`/`-Oz`**: Size-focused optimization
- **`-ffunction-sections`/`-fdata-sections`**: Enable linker garbage collection
- **`-fno-rtti`**: Remove C++ runtime type information (~10% smaller)
- **`-fno-exceptions`**: Remove exception handling (~15% smaller)
- **`--gc-sections`**: Strip unused code sections
- **`strip --strip-unneeded`**: Remove symbols and debug info
- **Optional**: Disable compression libraries (-30% size)

## 📚 Documentation

- [BUILD_ROCKSDB.md](BUILD_ROCKSDB.md) - Detailed build instructions
- [.github/copilot-instructions.md](.github/copilot-instructions.md) - Developer guide

## 🛠️ Workflows

### `build-rocksdb.yml` (Standard)
- ✅ Automatic on push/PR
- ✅ Caches RocksDB source
- ✅ Tests built library
- ✅ Creates releases on tags
- 🕐 ~10 minute build time
- 📦 30-day artifact retention

### `build-rocksdb-advanced.yml` (Advanced)
- 🎮 Manual trigger only
- ⚙️ Fully configurable
- 📊 Size comparison
- 🔍 Library analysis
- 📦 90-day artifact retention

## 🔄 Integrating Built Libraries

After downloading an artifact:

```bash
# Extract artifact
unzip librocksdb-v10.9.0-size-optimized.zip

# Replace library
cp lib/librocksdb.a /path/to/your/project/lib/

# Build your project
cargo clean
cargo build --release
```

## ⚡ Performance vs Size Trade-offs

| Feature | Size Reduction | Performance Impact |
|---------|---------------|-------------------|
| No RTTI | ~10% smaller | None |
| No exceptions | ~15% smaller | Abort on error |
| No compression libs | ~30% smaller | No compression |
| `-Oz` vs `-Os` | ~20% smaller | 5-10% slower |
| LTO | ~10-15% smaller | Compile time +50% |

## 🐛 Troubleshooting

**"undefined reference to rocksdb_*"**
→ Ensure `build.rs` links C++ stdlib and librocksdb.a

**Library too large**
→ Use advanced workflow with compression disabled and `-Oz`

**Tests fail after update**
→ Verify RocksDB version matches, run `cargo clean && cargo build`

## 📝 Example: Custom Size-Optimized Build

```bash
# Clone RocksDB
git clone --depth 1 --branch v10.9.0 https://github.com/facebook/rocksdb.git

cd rocksdb

# Maximum size optimization
export EXTRA_CXXFLAGS="-Oz -flto -ffunction-sections -fdata-sections -fno-exceptions -fno-rtti -DNDEBUG"
export EXTRA_LDFLAGS="-Wl,--gc-sections -Wl,--strip-all -flto"

# Build with compression disabled
make static_lib -j$(nproc) \
  DEBUG_LEVEL=0 \
  USE_RTTI=0 \
  PORTABLE=1 \
  DISABLE_SNAPPY=1 \
  DISABLE_ZLIB=1 \
  DISABLE_BZIP=1 \
  DISABLE_LZ4=1 \
  DISABLE_ZSTD=1 \
  EXTRA_CXXFLAGS="$EXTRA_CXXFLAGS" \
  EXTRA_LDFLAGS="$EXTRA_LDFLAGS"

strip --strip-unneeded librocksdb.a
# Result: ~8-10MB library
```

## 🔗 Resources

- [RocksDB Official Repo](https://github.com/facebook/rocksdb)
- [RocksDB Build Guide](https://github.com/facebook/rocksdb/blob/main/INSTALL.md)
- [GCC Optimization Options](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
