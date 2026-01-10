// Example demonstrating column family usage in RocksDB
// Column families allow logical partitioning of data within a single database

use rust_small_rocksdb::{DB, Options};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "/tmp/column_family_demo";

    // Clean up any existing database
    let _ = fs::remove_dir_all(path);

    println!("=== RocksDB Column Family Demo ===\n");

    // Step 1: Create database and column families
    println!("Step 1: Creating database with column families...");
    {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;

        let cf_opts = Options::default();
        let users_cf = db.create_column_family(&cf_opts, "users")?;
        let posts_cf = db.create_column_family(&cf_opts, "posts")?;
        let comments_cf = db.create_column_family(&cf_opts, "comments")?;

        println!("  ✓ Created 'users' column family");
        println!("  ✓ Created 'posts' column family");
        println!("  ✓ Created 'comments' column family\n");

        // Step 2: Write data to column families
        println!("Step 2: Writing data to column families...");

        // Users data
        db.put_cf(&users_cf, b"user:1", b"Alice Johnson")?;
        db.put_cf(&users_cf, b"user:2", b"Bob Smith")?;
        db.put_cf(&users_cf, b"user:3", b"Carol Davis")?;
        println!("  ✓ Added 3 users to 'users' CF");

        // Posts data
        db.put_cf(&posts_cf, b"post:1", b"Introduction to RocksDB")?;
        db.put_cf(&posts_cf, b"post:2", b"Column Families Explained")?;
        println!("  ✓ Added 2 posts to 'posts' CF");

        // Comments data
        db.put_cf(&comments_cf, b"comment:1:1", b"Great article!")?;
        db.put_cf(&comments_cf, b"comment:1:2", b"Very helpful, thanks!")?;
        println!("  ✓ Added 2 comments to 'comments' CF\n");

        // Step 3: Read data from column families
        println!("Step 3: Reading data from column families...");

        if let Some(user) = db.get_cf(&users_cf, b"user:1")? {
            println!("  User 1: {}", String::from_utf8_lossy(&user));
        }

        if let Some(post) = db.get_cf(&posts_cf, b"post:1")? {
            println!("  Post 1: {}", String::from_utf8_lossy(&post));
        }

        if let Some(comment) = db.get_cf(&comments_cf, b"comment:1:1")? {
            println!("  Comment: {}", String::from_utf8_lossy(&comment));
        }
        println!();

        // Step 4: Demonstrate isolation between CFs
        println!("Step 4: Demonstrating CF isolation...");
        db.put_cf(&users_cf, b"shared_key", b"user data")?;
        db.put_cf(&posts_cf, b"shared_key", b"post data")?;

        let user_val = db.get_cf(&users_cf, b"shared_key")?.unwrap();
        let post_val = db.get_cf(&posts_cf, b"shared_key")?.unwrap();

        println!("  Same key 'shared_key' in different CFs:");
        println!("    - users CF: {}", String::from_utf8_lossy(&user_val));
        println!("    - posts CF: {}", String::from_utf8_lossy(&post_val));
        println!("  ✓ Column families are isolated!\n");

        // Step 5: Delete from column family
        println!("Step 5: Deleting data from column families...");
        db.delete_cf(&comments_cf, b"comment:1:2")?;
        let deleted = db.get_cf(&comments_cf, b"comment:1:2")?;
        println!("  ✓ Deleted comment:1:2, value is now: {:?}\n", deleted);

        drop(users_cf);
        drop(posts_cf);
        drop(comments_cf);
        drop(db);
    }

    // Step 6: Reopen with column families
    println!("Step 6: Reopening database with existing column families...");
    {
        let opts = Options::default();
        let cf_names = vec!["default", "users", "posts", "comments"];
        let cf_opts = vec![
            Options::default(),
            Options::default(),
            Options::default(),
            Options::default(),
        ];

        let (db, cf_handles) = DB::open_with_column_families(&opts, path, &cf_names, &cf_opts)?;

        println!(
            "  ✓ Reopened database with {} column families",
            cf_handles.len()
        );

        // Verify data persists
        if let Some(user) = db.get_cf(&cf_handles[1], b"user:1")? {
            println!(
                "  ✓ Data persisted: User 1 = {}",
                String::from_utf8_lossy(&user)
            );
        }

        if let Some(post) = db.get_cf(&cf_handles[2], b"post:2")? {
            println!(
                "  ✓ Data persisted: Post 2 = {}",
                String::from_utf8_lossy(&post)
            );
        }
        println!();

        drop(cf_handles);
        drop(db);
    }

    // Step 7: Drop a column family
    println!("Step 7: Dropping a column family...");
    {
        let opts = Options::default();
        let cf_names = vec!["default", "users", "posts", "comments"];
        let cf_opts = vec![
            Options::default(),
            Options::default(),
            Options::default(),
            Options::default(),
        ];

        let (db, mut cf_handles) = DB::open_with_column_families(&opts, path, &cf_names, &cf_opts)?;

        // Drop the comments column family
        let comments_handle = cf_handles.pop().unwrap();
        db.drop_column_family(comments_handle)?;

        println!("  ✓ Dropped 'comments' column family");
        println!("    All data in that column family is now deleted\n");

        drop(cf_handles);
        drop(db);
    }

    println!("=== Summary ===");
    println!("Column families provide:");
    println!("  • Logical data partitioning within a single DB");
    println!("  • Isolation: same key can exist in multiple CFs with different values");
    println!("  • Independent configuration per family");
    println!("  • Efficient atomic writes across families");
    println!("  • Quick deletion of entire data partitions (drop CF)");
    println!();
    println!("Cleaned up demo database at: {}", path);

    // Clean up
    let _ = fs::remove_dir_all(path);

    Ok(())
}
