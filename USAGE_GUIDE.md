# Using rust_small_rocksdb in Your Project

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust_small_rocksdb = { git = "https://github.com/jessethepro/rust_small_rocksdb.git" }
```

Or if you want a specific commit:

```toml
[dependencies]
rust_small_rocksdb = { git = "https://github.com/jessethepro/rust_small_rocksdb.git", rev = "d4a3ef4" }
```

## Quick Start

```rust
use rust_small_rocksdb::{DB, Options};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    
    let db = DB::open(&opts, "/path/to/db")?;
    
    // Basic operations
    db.put(b"key", b"value")?;
    let value = db.get(b"key")?;
    db.delete(b"key")?;
    
    Ok(())
}
```

## Usage in Your Application

### 1. Basic Key-Value Store

```rust
use rust_small_rocksdb::{DB, Options};

pub struct MyDataStore {
    db: DB,
}

impl MyDataStore {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(MyDataStore { db })
    }
    
    pub fn set(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        self.db.put(key, value)?;
        Ok(())
    }
    
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        Ok(self.db.get(key)?)
    }
}
```

### 2. Multi-Tenant with Column Families

```rust
use rust_small_rocksdb::{DB, Options, ColumnFamilyHandle};

pub struct MultiTenantStore {
    db: DB,
    tenant_cfs: std::collections::HashMap<String, ColumnFamilyHandle>,
}

impl MultiTenantStore {
    pub fn add_tenant(&mut self, tenant_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let cf_opts = Options::default();
        let handle = self.db.create_column_family(&cf_opts, tenant_id)?;
        self.tenant_cfs.insert(tenant_id.to_string(), handle);
        Ok(())
    }
    
    pub fn set_for_tenant(&self, tenant_id: &str, key: &[u8], value: &[u8]) 
        -> Result<(), Box<dyn std::error::Error>> 
    {
        if let Some(cf) = self.tenant_cfs.get(tenant_id) {
            self.db.put_cf(cf, key, value)?;
        }
        Ok(())
    }
}
```

### 3. Configuration Store

```rust
use rust_small_rocksdb::{DB, Options};
use serde::{Serialize, Deserialize};

pub struct ConfigStore {
    db: DB,
}

impl ConfigStore {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(ConfigStore { db })
    }
    
    pub fn set_config<T: Serialize>(&self, key: &str, value: &T) 
        -> Result<(), Box<dyn std::error::Error>> 
    {
        let json = serde_json::to_vec(value)?;
        self.db.put(key.as_bytes(), &json)?;
        Ok(())
    }
    
    pub fn get_config<T: for<'de> Deserialize<'de>>(&self, key: &str) 
        -> Result<Option<T>, Box<dyn std::error::Error>> 
    {
        if let Some(data) = self.db.get(key.as_bytes())? {
            let value = serde_json::from_slice(&data)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}
```

### 4. Cache Implementation

```rust
use rust_small_rocksdb::{DB, Options};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Cache {
    db: DB,
}

impl Cache {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path)?;
        Ok(Cache { db })
    }
    
    pub fn set_with_ttl(&self, key: &[u8], value: &[u8], ttl_secs: u64) 
        -> Result<(), Box<dyn std::error::Error>> 
    {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let expiry = now + ttl_secs;
        
        // Store expiry time with the value
        let mut data = expiry.to_be_bytes().to_vec();
        data.extend_from_slice(value);
        
        self.db.put(key, &data)?;
        Ok(())
    }
    
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        if let Some(data) = self.db.get(key)? {
            if data.len() < 8 {
                return Ok(None);
            }
            
            let expiry = u64::from_be_bytes(data[0..8].try_into().unwrap());
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            
            if now < expiry {
                Ok(Some(data[8..].to_vec()))
            } else {
                // Expired, delete it
                self.db.delete(key)?;
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
```

## Building Your Application

The library includes a prebuilt RocksDB static library, so you can just:

```bash
cargo build
```

No additional dependencies or system packages required!

## Thread Safety

The `DB` type is `Send + Sync`, so you can safely share it across threads:

```rust
use std::sync::Arc;
use rust_small_rocksdb::{DB, Options};

let mut opts = Options::default();
opts.create_if_missing(true);
let db = Arc::new(DB::open(&opts, "/path/to/db")?);

// Use in multiple threads
let db_clone = Arc::clone(&db);
std::thread::spawn(move || {
    db_clone.put(b"key", b"value").unwrap();
});
```

## Error Handling

All operations return `Result<T, Error>`:

```rust
match db.get(b"key") {
    Ok(Some(value)) => println!("Found: {:?}", value),
    Ok(None) => println!("Key not found"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Performance Tips

1. **Reuse DB handle**: Create once, use everywhere (thread-safe)
2. **Batch operations**: Group related writes together
3. **Binary keys**: More efficient than string keys
4. **Column families**: Use for logical partitioning

## Examples

See the `examples/` directory in the repository:
- `basic.rs` - Basic usage
- `column_family_demo.rs` - Column family features

Run them with:
```bash
cargo run --example basic
cargo run --example column_family_demo
```

## Documentation

Full API documentation:
```bash
cargo doc --open
```

Or visit: https://github.com/jessethepro/rust_small_rocksdb

## Support

- GitHub Issues: https://github.com/jessethepro/rust_small_rocksdb/issues
- Documentation: [README.md](https://github.com/jessethepro/rust_small_rocksdb#readme)
- Column Families: [COLUMN_FAMILIES.md](https://github.com/jessethepro/rust_small_rocksdb/blob/master/COLUMN_FAMILIES.md)
