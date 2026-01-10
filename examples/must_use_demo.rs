// This example demonstrates the #[must_use] warnings
// Compile with: cargo build --example must_use_demo
// You should see warnings about unused values

use rust_small_rocksdb::{DB, Options};

fn main() {
    // This will warn: Options must be used to open a database
    Options::default();

    // This will warn: Database handle must be stored or the database will be immediately closed
    let mut opts = Options::default();
    opts.create_if_missing(true);
    DB::open(&opts, "/tmp/must_use_test").ok();

    // Correct usage:
    let db = DB::open(&opts, "/tmp/must_use_test_correct").expect("Failed to open DB");

    // This will warn: Iterators are lazy and do nothing unless consumed
    db.iter(rust_small_rocksdb::Direction::Forward);

    // Correct usage:
    for item in db.iter(rust_small_rocksdb::Direction::Forward) {
        let _entry = item.expect("Failed to read item");
    }
}
