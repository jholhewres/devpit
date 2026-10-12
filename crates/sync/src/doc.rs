//! What one namespace holds across every machine: records of fields, each
//! field the last value written to it and its stamp.
//!
//! Merging keeps, field by field, the value with the later stamp — the same
//! answer whichever machine merges, in whichever order, however often. A
//! value of `None` is a field cleared, and wins like any other write, so a
//! removal reaches the other machines too.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::clock::Stamp;

/// The shape a blob's words are written in. A machine that reads a later
/// one stops rather than write over what it does not understand.
pub const FORMAT: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    #[serde(rename = "v")]
    pub value: Option<String>,
    pub at: Stamp,
}

impl Cell {
    /// Whether this write is the one to keep over `other`. Two writes never
    /// share a stamp; were they to, the value settles it, everywhere alike.
    fn beats(&self, other: &Cell) -> bool {
        (&self.at, &self.value) > (&other.at, &other.value)
    }
}

pub type Fields = BTreeMap<String, Cell>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Doc {
    pub records: BTreeMap<String, Fields>,
}

impl Doc {
    pub fn get(&self, record: &str, field: &str) -> Option<&Cell> {
        self.records.get(record)?.get(field)
    }

    /// Keeps `cell` for `record`'s `field` when it beats what is there;
    /// whether it did.
    pub fn put(&mut self, record: &str, field: &str, cell: Cell) -> bool {
        let fields = self.records.entry(record.to_owned()).or_default();
        match fields.get(field) {
            Some(kept) if !cell.beats(kept) => false,
            _ => {
                fields.insert(field.to_owned(), cell);
                true
            }
        }
    }

    /// Takes every write of `other` that beats this one's; whether any did.
    pub fn merge(&mut self, other: &Doc) -> bool {
        let mut changed = false;
        for (record, fields) in &other.records {
            for (field, cell) in fields {
                changed |= self.put(record, field, cell.clone());
            }
        }
        changed
    }

    pub fn stamps(&self) -> impl Iterator<Item = &Stamp> {
        self.records
            .values()
            .flat_map(|fields| fields.values().map(|cell| &cell.at))
    }

    /// The words a blob holds.
    pub fn to_words(&self) -> Vec<u8> {
        serde_json::to_vec(&Words {
            format: FORMAT,
            records: self.records.clone(),
        })
        .unwrap_or_default()
    }

    /// The doc a blob's words hold; refused when they are in a later shape
    /// than this devpit writes, or none at all.
    pub fn from_words(words: &[u8]) -> Result<Doc, String> {
        let read: Words = serde_json::from_slice(words)
            .map_err(|_| "a synced blob opened and holds nothing devpit reads".to_owned())?;
        if read.format > FORMAT {
            return Err(
                "another machine syncs with a newer devpit: update this one to sync again"
                    .to_owned(),
            );
        }
        Ok(Doc {
            records: read.records,
        })
    }

    /// The same for two docs that hold the same writes, and only then.
    pub fn digest(&self) -> String {
        crate::key::hex(&Sha256::digest(self.to_words()))
    }
}

#[derive(Serialize, Deserialize)]
struct Words {
    format: u32,
    records: BTreeMap<String, Fields>,
}

#[cfg(test)]
#[path = "doc_tests.rs"]
mod tests;
