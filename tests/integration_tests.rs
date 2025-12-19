use rust_small_rocksdb::{DB, Options};
use std::fs;

#[test]
fn test_open_and_close() {
    let path = "/tmp/rust_rocksdb_test_open";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");
    assert_eq!(db.path(), path);

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_put_and_get() {
    let path = "/tmp/rust_rocksdb_test_put_get";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Put a value
    db.put(b"test_key", b"test_value")
        .expect("Failed to put value");

    // Get the value back
    let value = db.get(b"test_key").expect("Failed to get value");
    assert_eq!(value.as_deref(), Some(&b"test_value"[..]));

    // Get non-existent key
    let missing = db.get(b"missing_key").expect("Failed to get value");
    assert_eq!(missing, None);

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_delete() {
    let path = "/tmp/rust_rocksdb_test_delete";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Put a value
    db.put(b"delete_key", b"delete_value")
        .expect("Failed to put value");

    // Verify it exists
    let value = db.get(b"delete_key").expect("Failed to get value");
    assert_eq!(value.as_deref(), Some(&b"delete_value"[..]));

    // Delete it
    db.delete(b"delete_key").expect("Failed to delete key");

    // Verify it's gone
    let value = db.get(b"delete_key").expect("Failed to get value");
    assert_eq!(value, None);

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_multiple_operations() {
    let path = "/tmp/rust_rocksdb_test_multiple";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Put multiple values
    for i in 0..10 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        db.put(key.as_bytes(), value.as_bytes())
            .expect("Failed to put value");
    }

    // Read them back
    for i in 0..10 {
        let key = format!("key_{}", i);
        let expected = format!("value_{}", i);
        let value = db.get(key.as_bytes()).expect("Failed to get value");
        assert_eq!(value.as_deref(), Some(expected.as_bytes()));
    }

    // Delete some
    for i in 0..5 {
        let key = format!("key_{}", i);
        db.delete(key.as_bytes()).expect("Failed to delete key");
    }

    // Verify deleted
    for i in 0..5 {
        let key = format!("key_{}", i);
        let value = db.get(key.as_bytes()).expect("Failed to get value");
        assert_eq!(value, None);
    }

    // Verify remaining
    for i in 5..10 {
        let key = format!("key_{}", i);
        let expected = format!("value_{}", i);
        let value = db.get(key.as_bytes()).expect("Failed to get value");
        assert_eq!(value.as_deref(), Some(expected.as_bytes()));
    }

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_binary_data() {
    let path = "/tmp/rust_rocksdb_test_binary";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Test with binary data including null bytes
    let key = b"\x00\x01\x02\xff\xfe";
    let value = b"\x00\x00\xff\xff\x12\x34";

    db.put(key, value).expect("Failed to put binary value");

    let retrieved = db.get(key).expect("Failed to get binary value");
    assert_eq!(retrieved.as_deref(), Some(&value[..]));

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_iterator_forward() {
    use rust_small_rocksdb::Direction;

    let path = "/tmp/rust_rocksdb_test_iterator_forward";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Insert test data
    db.put(b"key1", b"value1").unwrap();
    db.put(b"key2", b"value2").unwrap();
    db.put(b"key3", b"value3").unwrap();

    // Iterate forward
    let mut items: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for item in db.iter(Direction::Forward) {
        let (key, value) = item.unwrap();
        items.push((key.to_vec(), value.to_vec()));
    }

    assert_eq!(items.len(), 3);
    assert_eq!(items[0], (b"key1".to_vec(), b"value1".to_vec()));
    assert_eq!(items[1], (b"key2".to_vec(), b"value2".to_vec()));
    assert_eq!(items[2], (b"key3".to_vec(), b"value3".to_vec()));

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_iterator_reverse() {
    use rust_small_rocksdb::Direction;

    let path = "/tmp/rust_rocksdb_test_iterator_reverse";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Insert test data
    db.put(b"key1", b"value1").unwrap();
    db.put(b"key2", b"value2").unwrap();
    db.put(b"key3", b"value3").unwrap();

    // Iterate in reverse
    let mut items: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for item in db.iter(Direction::Reverse) {
        let (key, value) = item.unwrap();
        items.push((key.to_vec(), value.to_vec()));
    }

    assert_eq!(items.len(), 3);
    assert_eq!(items[0], (b"key3".to_vec(), b"value3".to_vec()));
    assert_eq!(items[1], (b"key2".to_vec(), b"value2".to_vec()));
    assert_eq!(items[2], (b"key1".to_vec(), b"value1".to_vec()));

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_raw_iterator() {
    let path = "/tmp/rust_rocksdb_test_raw_iterator";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Insert test data
    db.put(b"aaa", b"value1").unwrap();
    db.put(b"bbb", b"value2").unwrap();
    db.put(b"ccc", b"value3").unwrap();

    // Test seek
    let mut iter = db.raw_iterator();
    iter.seek(b"bbb");

    assert!(iter.valid());
    assert_eq!(iter.key(), Some(&b"bbb"[..]));
    assert_eq!(iter.value(), Some(&b"value2"[..]));

    // Test next
    iter.next();
    assert!(iter.valid());
    assert_eq!(iter.key(), Some(&b"ccc"[..]));

    // Test prev
    iter.prev();
    assert!(iter.valid());
    assert_eq!(iter.key(), Some(&b"bbb"[..]));

    // Test seek_to_first
    iter.seek_to_first();
    assert!(iter.valid());
    assert_eq!(iter.key(), Some(&b"aaa"[..]));

    // Test seek_to_last
    iter.seek_to_last();
    assert!(iter.valid());
    assert_eq!(iter.key(), Some(&b"ccc"[..]));

    // Drop iterator before DB
    drop(iter);
    drop(db);
    let _ = fs::remove_dir_all(path);
}
