// Basic usage example for rust_small_rocksdb
//
// This example demonstrates the core functionality:
// - Opening a database
// - Putting key-value pairs
// - Getting values
// - Deleting keys

use rust_small_rocksdb::{DB, Options};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Clean up any existing database
    let path = "/tmp/basic_example";
    let _ = std::fs::remove_dir_all(path);

    println!("=== Basic RocksDB Usage ===\n");

    // Configure database options
    let mut opts = Options::default();
    opts.create_if_missing(true);

    // Open the database
    let db = DB::open(&opts, path)?;
    println!("✓ Opened database at: {}\n", path);

    // Put some data
    println!("Writing data...");
    db.put(b"name", b"RocksDB")?;
    db.put(b"type", b"Key-Value Store")?;
    db.put(b"language", b"C++")?;
    db.put(b"bindings", b"Rust")?;
    println!("✓ Wrote 4 key-value pairs\n");

    // Get data
    println!("Reading data...");
    if let Some(value) = db.get(b"name")? {
        println!("  name = {}", String::from_utf8_lossy(&value));
    }
    if let Some(value) = db.get(b"type")? {
        println!("  type = {}", String::from_utf8_lossy(&value));
    }
    if let Some(value) = db.get(b"language")? {
        println!("  language = {}", String::from_utf8_lossy(&value));
    }
    if let Some(value) = db.get(b"bindings")? {
        println!("  bindings = {}", String::from_utf8_lossy(&value));
    }
    println!();

    // Delete a key
    println!("Deleting key 'language'...");
    db.delete(b"language")?;
    let deleted = db.get(b"language")?;
    println!("✓ Value after deletion: {:?}\n", deleted);

    // Check non-existent key
    println!("Checking non-existent key...");
    let result = db.get(b"nonexistent")?;
    println!("✓ Non-existent key returns: {:?}\n", result);

    // Binary data support
    println!("Testing binary data...");
    let binary_key = b"\x00\x01\x02\xff\xfe";
    let binary_value = b"\xde\xad\xbe\xef";
    db.put(binary_key, binary_value)?;
    let retrieved = db.get(binary_key)?;
    println!("✓ Binary data: {:?}", retrieved);
    println!();

    println!("Database operations completed successfully!");

    // Clean up
    drop(db);
    let _ = std::fs::remove_dir_all(path);

    Ok(())
}
