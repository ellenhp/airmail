use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use log::warn;
use redb::{Database, ReadableTable, TableDefinition};

use crate::error::IndexerError;

const TABLE_AREAS: TableDefinition<u64, &[u8]> = TableDefinition::new("admin_areas");
const TABLE_NAMES: TableDefinition<u64, &str> = TableDefinition::new("admin_names");
const TABLE_LANGS: TableDefinition<u64, &str> = TableDefinition::new("admin_langs");
const BUFFER_SIZE: usize = 5000;

/// A cache for storing administrative area information.
pub struct IndexerCache {
    database: Database,
    buffer: Arc<RwLock<VecDeque<WofCacheItem>>>,
}

impl IndexerCache {
    /// Initialise a new indexer cache
    pub fn new(redb_path: &Path) -> Result<Self> {
        let database = Database::create(redb_path)?;
        let txn = database.begin_write()?;
        txn.open_table(TABLE_AREAS)?;
        txn.open_table(TABLE_NAMES)?;
        txn.open_table(TABLE_LANGS)?;
        txn.commit()?;

        Ok(Self {
            database,
            buffer: Arc::new(RwLock::new(VecDeque::new())),
        })
    }

    /// Lookup a cell id in the cache and return the admin ids
    pub fn query_area(&self, cell_id: u64) -> Result<Vec<u64>> {
        let mut ids: Vec<u64> = Vec::new();
        let txn = self.database.begin_read()?;
        let table = txn.open_table(TABLE_AREAS)?;
        if let Some(admin_ids) = table.get(&cell_id)? {
            for admin_id in admin_ids.value().chunks(8) {
                ids.push(u64::from_le_bytes([
                    admin_id[0],
                    admin_id[1],
                    admin_id[2],
                    admin_id[3],
                    admin_id[4],
                    admin_id[5],
                    admin_id[6],
                    admin_id[7],
                ]));
            }
        }
        Ok(ids)
    }

    /// Lookup an admin id in the cache and return the names
    pub fn query_names_cache(&self, admin: u64) -> Result<Vec<String>> {
        let txn = self.database.begin_read()?;
        let table = txn.open_table(TABLE_NAMES)?;
        if let Some(names_ref) = table.get(admin)? {
            let names = names_ref.value().to_string();
            let names = names
                .split('\0')
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            return Ok(names);
        }
        Err(IndexerError::NoNamesFound.into())
    }

    /// Lookup an admin id in the cache and return the languages
    pub fn query_languages_cache(&self, admin: u64) -> Result<Vec<String>> {
        let txn = self.database.begin_read()?;
        let table = txn.open_table(TABLE_LANGS)?;
        if let Some(langs_ref) = table.get(admin)? {
            let langs = langs_ref.value().to_string();
            let langs = langs
                .split('\0')
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            return Ok(langs);
        }
        Err(IndexerError::NoLangsFound.into())
    }

    /// Write an item to the cache, items will be written to a buffer
    /// and flushed to the database when the buffer is full.
    pub fn buffered_write_item(&self, item: WofCacheItem) -> Result<()> {
        let mut buffer = self.buffer.write().unwrap();
        buffer.push_back(item);

        if buffer.len() >= BUFFER_SIZE {
            self.flush_buffer(&mut buffer)?;
        }
        Ok(())
    }

    /// Flush the buffer to the database
    fn flush_buffer(&self, buffer: &mut VecDeque<WofCacheItem>) -> Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        let write = self.database.begin_write()?;
        for item in buffer.drain(..) {
            match item {
                WofCacheItem::Names(admin, names) => {
                    let mut table = write.open_table(TABLE_NAMES)?;
                    let packed = names.join("\0");
                    table.insert(admin, packed.as_str())?;
                }
                WofCacheItem::Langs(admin, langs) => {
                    let mut table = write.open_table(TABLE_LANGS)?;
                    let packed = langs.join("\0");
                    table.insert(admin, packed.as_str())?;
                }
                WofCacheItem::Admins(s2cell, admins) => {
                    let mut table = write.open_table(TABLE_AREAS)?;
                    let packed = admins
                        .iter()
                        .flat_map(|id| id.to_le_bytes())
                        .collect::<Vec<_>>();
                    table.insert(s2cell, packed.as_slice())?;
                }
            }
        }
        write.commit()?;
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        let mut buffer = self.buffer.write().unwrap();
        self.flush_buffer(&mut buffer)
    }
}

/// Drop implementation for the cache, flushes the buffer to the database
/// as there is no guarantee that the buffer will be flushed before the cache is dropped.
impl Drop for IndexerCache {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            warn!("Failed to flush cache: {}", e);
        }
    }
}
pub enum WofCacheItem {
    Names(u64, Vec<String>),
    Langs(u64, Vec<String>),
    Admins(u64, Vec<u64>),
}
