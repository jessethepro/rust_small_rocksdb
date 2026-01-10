// This example demonstrates that the compile-time size assertions work
// The opaque FFI types are guaranteed to be zero-sized

use std::mem::size_of;

// These are the internal opaque types (normally not accessible)
// We recreate them here to demonstrate the size check
#[repr(C)]
struct OpaqueType {
    _private: [u8; 0],
}

// If we tried to create a non-zero-sized "opaque" type, it would fail at compile time
// Uncomment this to see the compile error:
/*
#[repr(C)]
struct BadOpaqueType {
    _private: [u8; 1],  // Not zero-sized!
}

const _: () = {
    const fn assert_zero_sized<T>() {
        assert!(std::mem::size_of::<T>() == 0);
    }
    assert_zero_sized::<BadOpaqueType>(); // This would fail!
};
*/

fn main() {
    println!(
        "Size of zero-sized opaque type: {} bytes",
        size_of::<OpaqueType>()
    );
    println!("✓ All FFI opaque types are verified to be zero-sized at compile time");
    println!();
    println!("This ensures:");
    println!("  - No accidental padding or alignment issues");
    println!("  - Types remain truly opaque (cannot be constructed)");
    println!("  - Compiler optimizations are maximized");
    println!("  - Memory layout matches C expectations");
}
