use std::{
    collections::VecDeque,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
};

use anyhow::Result;
use log::{trace, warn};
use redb::{Database, ReadableTable, TableDefinition};

const TABLE_AREAS: TableDefinition<u64, &[u8]> = TableDefinition::new("admin_areas");
const TABLE_NAMES: TableDefinition<u64, &str> = TableDefinition::new("admin_names");
const TABLE_LANGS: TableDefinition<u64, &str> = TableDefinition::new("admin_langs");
const TABLE_NODE_LOCATION: TableDefinition<i64, (f64, f64)> =
    TableDefinition::new("admin_node_location");
pub const BUFFER_SIZE: usize = 25000;

/// A cache for storing administrative area information.
pub struct IndexerCache {
    database: Database,
    buffer_size: AtomicUsize,
    buffer: Arc<RwLock<VecDeque<WofCacheItem>>>,
}

impl IndexerCache {
    /// Initialise a new indexer cache
    pub fn new(redb_path: &Path) -> Result<Self> {
        trace!("Opening cache database at {:?}", redb_path);
        let database = Database::create(redb_path)?;
        let txn = database.begin_write()?;
        txn.open_table(TABLE_AREAS)?;
        txn.open_table(TABLE_NAMES)?;
        txn.open_table(TABLE_LANGS)?;
        txn.open_table(TABLE_NODE_LOCATION)?;
        txn.commit()?;

        Ok(Self {
            database,
            buffer_size: AtomicUsize::new(BUFFER_SIZE),
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
    pub fn query_names_cache(&self, admin: u64) -> Result<Option<Vec<String>>> {
        let txn = self.database.begin_read()?;
        let table = txn.open_table(TABLE_NAMES)?;
        if let Some(names_ref) = table.get(admin)? {
            let names = names_ref.value().to_string();
            let names = names
                .split('\0')
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            return Ok(Some(names));
        }
        Ok(None)
    }

    /// Lookup an admin id in the cache and return the languages
    pub fn query_languages_cache(&self, admin: u64) -> Result<Option<Vec<String>>> {
        let txn = self.database.begin_read()?;
        let table = txn.open_table(TABLE_LANGS)?;
        if let Some(langs_ref) = table.get(admin)? {
            let langs = langs_ref.value().to_string();
            let langs = langs
                .split('\0')
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            return Ok(Some(langs));
        }
        Ok(None)
    }

    /// Lookup a node id in the cache and return the location
    pub fn query_node_location(&self, node_id: i64) -> Result<Option<(f64, f64)>> {
        let txn = self.database.begin_read()?;
        let table = txn.open_table(TABLE_NODE_LOCATION)?;
        if let Some(location) = table.get(node_id)? {
            return Ok(Some(location.value()));
        }
        Ok(None)
    }

    /// Write an item to the cache, items will be written to a buffer
    /// and flushed to the database when the buffer is full.
    pub fn buffered_write_item(&self, item: WofCacheItem) -> Result<()> {
        let mut buffer = self.buffer.write().unwrap();
        buffer.push_back(item);

        if buffer.len() >= self.buffer_size.load(Ordering::Relaxed) {
            self.flush_buffer(&mut buffer)?;
        }
        Ok(())
    }

    /// Flush the buffer to the database
    fn flush_buffer(&self, buffer: &mut VecDeque<WofCacheItem>) -> Result<()> {
        if buffer.is_empty() {
            return Ok(());
        }

        trace!("Flushing cache buffer");

        let write = self.database.begin_write()?;
        {
            let mut names_table = write.open_table(TABLE_NAMES)?;
            let mut langs_table = write.open_table(TABLE_LANGS)?;
            let mut areas_table = write.open_table(TABLE_AREAS)?;
            let mut locations_tabls = write.open_table(TABLE_NODE_LOCATION)?;

            for item in buffer.drain(..) {
                match item {
                    WofCacheItem::Names(admin, names) => {
                        let packed = names.join("\0");
                        names_table.insert(admin, packed.as_str())?;
                    }
                    WofCacheItem::Langs(admin, langs) => {
                        let packed = langs.join("\0");
                        langs_table.insert(admin, packed.as_str())?;
                    }
                    WofCacheItem::Admins(s2cell, admins) => {
                        let packed = admins
                            .iter()
                            .flat_map(|id| id.to_le_bytes())
                            .collect::<Vec<_>>();
                        areas_table.insert(s2cell, packed.as_slice())?;
                    }
                    WofCacheItem::NodeLocation(node_id, location) => {
                        locations_tabls.insert(node_id, location)?;
                    }
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

    /// Set the buffer size for the cache and flush the buffer
    pub fn buffer_size(&self, size: usize) -> Result<()> {
        self.buffer_size.store(size, Ordering::Relaxed);
        self.flush()
    }

    /// Reset the buffer size to the default value
    pub fn buffer_size_default(&self) -> Result<()> {
        self.buffer_size.store(BUFFER_SIZE, Ordering::Relaxed);
        self.flush()
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
    NodeLocation(i64, (f64, f64)),
}
