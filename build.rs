use std::env;
use std::path::PathBuf;

fn main() {
    // Get the project root directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let lib_path = PathBuf::from(&manifest_dir).join("lib");

    // Tell cargo to look for the static library in lib/
    println!("cargo:rustc-link-search=native={}", lib_path.display());

    // Link the RocksDB static library
    println!("cargo:rustc-link-lib=static=rocksdb");

    // Link C++ standard library (required for RocksDB)
    // On Linux, use libstdc++; on macOS, use libc++
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=stdc++");
    } else if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
    }

    // Re-run the build script if the library changes
    println!("cargo:rerun-if-changed=lib/librocksdb.a");
    println!("cargo:rerun-if-changed=build.rs");
}
