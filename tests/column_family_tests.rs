use rust_small_rocksdb::{DB, Options};
use std::fs;

#[test]
fn test_create_column_family() {
    let path = "/tmp/rust_rocksdb_test_cf";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Create a column family
    let cf_opts = Options::default();
    let cf_handle = db
        .create_column_family(&cf_opts, "test_cf")
        .expect("Failed to create column family");

    // Column family handle is created successfully
    // (we can't access the inner pointer as it's private, which is correct)

    drop(cf_handle);
    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_create_multiple_column_families() {
    let path = "/tmp/rust_rocksdb_test_multiple_cf";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Create multiple column families with separate scopes
    let cf_opts = Options::default();

    {
        let _cf1 = db
            .create_column_family(&cf_opts, "users")
            .expect("Failed to create users CF");
    }

    {
        let _cf2 = db
            .create_column_family(&cf_opts, "posts")
            .expect("Failed to create posts CF");
    }

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_column_family_invalid_name() {
    let path = "/tmp/rust_rocksdb_test_cf_invalid";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Try to create a column family with an invalid name (embedded null)
    let cf_opts = Options::default();
    let result = db.create_column_family(&cf_opts, "test\0invalid");

    assert!(result.is_err());

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_put_get_cf() {
    let path = "/tmp/rust_rocksdb_test_put_get_cf";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Create a column family
    let cf_opts = Options::default();
    let cf_handle = db
        .create_column_family(&cf_opts, "users")
        .expect("Failed to create column family");

    // Put data in the column family
    db.put_cf(&cf_handle, b"user:1", b"Alice")
        .expect("Failed to put in CF");
    db.put_cf(&cf_handle, b"user:2", b"Bob")
        .expect("Failed to put in CF");

    // Get data from the column family
    let value1 = db.get_cf(&cf_handle, b"user:1").expect("Failed to get");
    assert_eq!(value1.as_deref(), Some(&b"Alice"[..]));

    let value2 = db.get_cf(&cf_handle, b"user:2").expect("Failed to get");
    assert_eq!(value2.as_deref(), Some(&b"Bob"[..]));

    // Get non-existent key
    let value3 = db.get_cf(&cf_handle, b"user:3").expect("Failed to get");
    assert_eq!(value3, None);

    drop(cf_handle);
    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_delete_cf() {
    let path = "/tmp/rust_rocksdb_test_delete_cf";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Create a column family
    let cf_opts = Options::default();
    let cf_handle = db
        .create_column_family(&cf_opts, "users")
        .expect("Failed to create column family");

    // Put and delete
    db.put_cf(&cf_handle, b"user:1", b"Alice")
        .expect("Failed to put");
    db.delete_cf(&cf_handle, b"user:1")
        .expect("Failed to delete");

    let value = db.get_cf(&cf_handle, b"user:1").expect("Failed to get");
    assert_eq!(value, None);

    drop(cf_handle);
    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_cf_isolation() {
    let path = "/tmp/rust_rocksdb_test_cf_isolation";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Create two column families
    let cf_opts = Options::default();
    let cf1 = db
        .create_column_family(&cf_opts, "cf1")
        .expect("Failed to create CF1");
    let cf2 = db
        .create_column_family(&cf_opts, "cf2")
        .expect("Failed to create CF2");

    // Put same key in both CFs with different values
    db.put_cf(&cf1, b"key", b"value_cf1")
        .expect("Failed to put in CF1");
    db.put_cf(&cf2, b"key", b"value_cf2")
        .expect("Failed to put in CF2");

    // Verify they're isolated
    let val1 = db.get_cf(&cf1, b"key").expect("Failed to get from CF1");
    let val2 = db.get_cf(&cf2, b"key").expect("Failed to get from CF2");

    assert_eq!(val1.as_deref(), Some(&b"value_cf1"[..]));
    assert_eq!(val2.as_deref(), Some(&b"value_cf2"[..]));

    drop(cf1);
    drop(cf2);
    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_drop_column_family() {
    let path = "/tmp/rust_rocksdb_test_drop_cf";
    let _ = fs::remove_dir_all(path);

    let mut opts = Options::default();
    opts.create_if_missing(true);

    let db = DB::open(&opts, path).expect("Failed to open database");

    // Create a column family
    let cf_opts = Options::default();
    let cf_handle = db
        .create_column_family(&cf_opts, "temp")
        .expect("Failed to create column family");

    // Put some data
    db.put_cf(&cf_handle, b"key", b"value")
        .expect("Failed to put");

    // Drop the column family
    db.drop_column_family(cf_handle).expect("Failed to drop CF");

    drop(db);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_open_with_column_families() {
    let path = "/tmp/rust_rocksdb_test_open_with_cf";
    let _ = fs::remove_dir_all(path);

    // First, create a database with column families
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let db = DB::open(&opts, path).expect("Failed to open database");

        let cf_opts = Options::default();
        let cf1 = db
            .create_column_family(&cf_opts, "users")
            .expect("Failed to create users CF");
        let cf2 = db
            .create_column_family(&cf_opts, "posts")
            .expect("Failed to create posts CF");

        // Put some data
        db.put_cf(&cf1, b"user:1", b"Alice").expect("Failed to put");
        db.put_cf(&cf2, b"post:1", b"Hello").expect("Failed to put");

        drop(cf1);
        drop(cf2);
        drop(db);
    }

    // Now reopen with column families
    {
        let opts = Options::default();
        let cf_names = vec!["default", "users", "posts"];
        let cf_opts = vec![Options::default(), Options::default(), Options::default()];

        let (db, cf_handles) = DB::open_with_column_families(&opts, path, &cf_names, &cf_opts)
            .expect("Failed to open with CFs");

        assert_eq!(cf_handles.len(), 3);

        // Verify data is still there
        let value1 = db
            .get_cf(&cf_handles[1], b"user:1")
            .expect("Failed to get from users CF");
        assert_eq!(value1.as_deref(), Some(&b"Alice"[..]));

        let value2 = db
            .get_cf(&cf_handles[2], b"post:1")
            .expect("Failed to get from posts CF");
        assert_eq!(value2.as_deref(), Some(&b"Hello"[..]));

        drop(cf_handles);
        drop(db);
    }

    let _ = fs::remove_dir_all(path);
}

#[test]
fn test_open_with_column_families_errors() {
    let path = "/tmp/rust_rocksdb_test_open_with_cf_errors";
    let _ = fs::remove_dir_all(path);

    let opts = Options::default();

    // Test mismatched lengths
    let cf_names = vec!["default", "users"];
    let cf_opts = vec![Options::default()]; // Only 1 option for 2 names

    let result = DB::open_with_column_families(&opts, path, &cf_names, &cf_opts);
    assert!(result.is_err());

    // Test empty names
    let cf_names: Vec<&str> = vec![];
    let cf_opts: Vec<Options> = vec![];

    let result = DB::open_with_column_families(&opts, path, &cf_names, &cf_opts);
    assert!(result.is_err());

    let _ = fs::remove_dir_all(path);
}
