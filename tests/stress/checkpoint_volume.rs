// SPDX-License-Identifier: MIT
//! Stress test: many checkpoints in sequence.

use redb::{Database, ReadableTable, TableDefinition};
use tempfile::NamedTempFile;

const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("checkpoints");

#[test]
fn test_1000_sequential_checkpoints() {
    let file = NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();

    let data = vec![0u8; 10_000]; // 10KB per checkpoint

    for i in 0..1000 {
        let write_txn = db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(TABLE).unwrap();
            let key = format!("cp_{i:06}");
            table.insert(key.as_str(), data.as_slice()).unwrap();
        }
        write_txn.commit().unwrap();
    }

    // Verify all checkpoints exist
    let read_txn = db.begin_read().unwrap();
    let table = read_txn.open_table(TABLE).unwrap();

    let count = table.iter().unwrap().count();
    assert_eq!(count, 1000);

    // DB file should be reasonable size
    let file_size = std::fs::metadata(file.path()).unwrap().len();
    assert!(
        file_size < 20_000_000,
        "DB file too large: {} bytes",
        file_size
    ); // < 20MB
}

#[test]
fn test_large_single_checkpoint() {
    let file = NamedTempFile::new().unwrap();
    let db = Database::create(file.path()).unwrap();

    // 5MB checkpoint (simulating 500k+ token conversation)
    let data = vec![b'x'; 5_000_000];

    let write_txn = db.begin_write().unwrap();
    {
        let mut table = write_txn.open_table(TABLE).unwrap();
        table.insert("large_checkpoint", data.as_slice()).unwrap();
    }
    write_txn.commit().unwrap();

    // Read it back
    let read_txn = db.begin_read().unwrap();
    let table = read_txn.open_table(TABLE).unwrap();
    let value = table.get("large_checkpoint").unwrap().unwrap();
    assert_eq!(value.value().len(), 5_000_000);
}
