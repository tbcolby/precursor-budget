//! PDDB storage for Budget Ledger.

extern crate alloc;
use alloc::vec::Vec;

use crate::ledger::Transaction;

const DICT: &str = "budget.data";
const KEY_TXNS: &str = "transactions";
const KEY_CONFIG: &str = "config";

pub struct Storage {
    pddb: pddb::Pddb,
}

impl Storage {
    pub fn new() -> Result<Self, ()> {
        let pddb = pddb::Pddb::new();
        pddb.is_mounted_blocking();
        Ok(Self { pddb })
    }

    fn read_key(&mut self, key: &str) -> Option<Vec<u8>> {
        let mut handle = self
            .pddb
            .get(DICT, key, None, false, false, None, None::<fn()>)
            .ok()?;
        let mut buf = Vec::new();
        use std::io::Read;
        handle.read_to_end(&mut buf).ok()?;
        Some(buf)
    }

    fn write_key(&mut self, key: &str, data: &[u8]) {
        if let Ok(mut handle) = self.pddb.get(
            DICT, key, None, true, true, Some(data.len()), None::<fn()>,
        ) {
            use std::io::{Seek, Write};
            handle.seek(std::io::SeekFrom::Start(0)).ok();
            handle.write_all(data).ok();
            handle.set_len(data.len() as u64).ok();
        }
        self.pddb.sync().ok();
    }

    pub fn load_transactions(&mut self) -> Vec<Transaction> {
        match self.read_key(KEY_TXNS) {
            Some(buf) => serde_json::from_slice(&buf).unwrap_or_default(),
            None => Vec::new(),
        }
    }

    pub fn save_transactions(&mut self, txns: &[Transaction]) {
        let data = serde_json::to_vec(txns).unwrap_or_default();
        self.write_key(KEY_TXNS, &data);
    }

    pub fn load_next_id(&mut self) -> u32 {
        let buf = match self.read_key(KEY_CONFIG) {
            Some(b) => b,
            None => return 1,
        };
        let json: serde_json::Value = match serde_json::from_slice(&buf) {
            Ok(j) => j,
            Err(_) => return 1,
        };
        json.get("next_id").and_then(|v| v.as_u64()).unwrap_or(1) as u32
    }

    pub fn save_next_id(&mut self, next_id: u32) {
        let json = serde_json::json!({ "next_id": next_id });
        let data = serde_json::to_vec(&json).unwrap_or_default();
        self.write_key(KEY_CONFIG, &data);
    }
}
