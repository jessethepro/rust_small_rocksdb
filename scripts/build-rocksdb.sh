#!/bin/bash
# Build RocksDB with size optimizations
# Usage: ./scripts/build-rocksdb.sh [version] [optimization]

set -e

# Configuration
ROCKSDB_VERSION="${1:-v10.7.5}"
OPTIMIZATION="${2:-Os}"
ROCKSDB_DIR="rocksdb-build"
OUTPUT_DIR="$(pwd)/lib"

echo "==================================="
echo "RocksDB Size-Optimized Build Script"
echo "==================================="
echo "Version: $ROCKSDB_VERSION"
echo "Optimization: -$OPTIMIZATION"
echo "==================================="

# Check dependencies
command -v git >/dev/null 2>&1 || { echo "Error: git not found"; exit 1; }
command -v make >/dev/null 2>&1 || { echo "Error: make not found"; exit 1; }
command -v g++ >/dev/null 2>&1 || { echo "Error: g++ not found"; exit 1; }

# Clone RocksDB if needed
if [ ! -d "$ROCKSDB_DIR" ]; then
    echo "Cloning RocksDB $ROCKSDB_VERSION..."
    git clone --depth 1 --branch "$ROCKSDB_VERSION" \
        https://github.com/facebook/rocksdb.git "$ROCKSDB_DIR"
else
    echo "Using existing RocksDB directory"
fi

cd "$ROCKSDB_DIR"

# Build flags for size optimization
export EXTRA_CXXFLAGS="-$OPTIMIZATION -ffunction-sections -fdata-sections -fno-exceptions -fno-rtti -DNDEBUG"
export EXTRA_LDFLAGS="-Wl,--gc-sections -Wl,--strip-all"

echo "Building RocksDB..."
make clean 2>/dev/null || true

make static_lib -j$(nproc) \
    DEBUG_LEVEL=0 \
    USE_RTTI=0 \
    DISABLE_WARNING_AS_ERROR=1 \
    PORTABLE=1 \
    FORCE_SSE42=0 \
    DISABLE_SNAPPY=1 \
    DISABLE_ZLIB=1 \
    DISABLE_BZIP=1 \
    DISABLE_LZ4=1 \
    DISABLE_ZSTD=1 \
    EXTRA_CXXFLAGS="$EXTRA_CXXFLAGS" \
    EXTRA_LDFLAGS="$EXTRA_LDFLAGS"

echo "Stripping library..."
strip --strip-unneeded librocksdb.a

# Show size
echo ""
echo "==================================="
echo "Build Complete!"
echo "==================================="
ls -lh librocksdb.a
echo "Library size: $(du -h librocksdb.a | cut -f1)"

# Copy to output
mkdir -p "$OUTPUT_DIR"
cp librocksdb.a "$OUTPUT_DIR/"
echo ""
echo "Library copied to: $OUTPUT_DIR/librocksdb.a"

# Create build info
cat > "$OUTPUT_DIR/build-info.txt" << EOF
RocksDB Version: $ROCKSDB_VERSION
Build Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
Optimization: -$OPTIMIZATION
CXXFLAGS: $EXTRA_CXXFLAGS
LDFLAGS: $EXTRA_LDFLAGS
Compression: Disabled (Snappy, Zlib, Bzip2, LZ4, Zstd)
Library Size: $(du -h "$OUTPUT_DIR/librocksdb.a" | cut -f1)
EOF

echo "Build info written to: $OUTPUT_DIR/build-info.txt"
echo ""
echo "Done! You can now run: cargo build"
