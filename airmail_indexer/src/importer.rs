use airmail::{
    index::AirmailIndex,
    poi::{SchemafiedPoi, ToIndexPoi},
};
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use futures_util::future::join_all;
use log::{info, trace, warn};
use redb::Database;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{spawn, task::spawn_blocking};

use crate::{
    pip_tree::PipTree,
    populate_admin_areas,
    wof::{ConcisePipResponse, WhosOnFirst},
    WofCacheItem, TABLE_AREAS, TABLE_LANGS, TABLE_NAMES,
};

pub struct ImporterBuilder {
    index: AirmailIndex,
    admin_cache_path: Option<PathBuf>,
    wof_db_path: PathBuf,
    pip_tree_path: Option<PathBuf>,
}

impl ImporterBuilder {
    pub fn new(airmail_index_path: &Path, wof_db_path: &Path) -> Result<Self> {
        // Create the index
        let index = AirmailIndex::create(airmail_index_path)?;

        Ok(Self {
            index,
            admin_cache_path: None,
            wof_db_path: wof_db_path.to_path_buf(),
            pip_tree_path: None,
        })
    }

    pub fn admin_cache(mut self, admin_cache: &Path) -> Self {
        self.admin_cache_path = Some(admin_cache.to_path_buf());
        self
    }

    pub fn pip_tree_cache(mut self, pip_tree_cache: &Path) -> Self {
        self.pip_tree_path = Some(pip_tree_cache.to_path_buf());
        self
    }

    pub async fn build(self) -> Result<Importer> {
        let admin_cache = if let Some(admin_cache) = self.admin_cache_path {
            admin_cache
        } else {
            std::env::temp_dir().join("admin_cache.db")
        };

        let db = Database::create(&admin_cache)?;
        {
            let txn = db.begin_write()?;
            {
                txn.open_table(TABLE_AREAS)?;
                txn.open_table(TABLE_NAMES)?;
            }
            txn.commit()?;
        }

        let wof_db = WhosOnFirst::new(&self.wof_db_path).await?;

        let pip_tree = if let Some(pip_tree_cache) = self.pip_tree_path {
            Some(PipTree::new_or_load(&wof_db, &pip_tree_cache).await?)
        } else {
            None
        };

        Importer::new(self.index, db, wof_db, pip_tree).await
    }
}

pub struct Importer {
    index: AirmailIndex,
    admin_cache: Arc<Database>,
    wof_db: WhosOnFirst,
    pip_tree: Option<PipTree<ConcisePipResponse>>,
}

impl Importer {
    pub async fn new(
        index: AirmailIndex,
        admin_cache: Database,
        wof_db: WhosOnFirst,
        pip_tree: Option<PipTree<ConcisePipResponse>>,
    ) -> Result<Self> {
        Ok(Self {
            index,
            admin_cache: Arc::new(admin_cache),
            wof_db,
            pip_tree,
        })
    }

    pub async fn run_import(mut self, source: &str, receiver: Receiver<ToIndexPoi>) -> Result<()> {
        let source = source.to_string();
        let (to_cache_sender, to_cache_receiver): (Sender<WofCacheItem>, Receiver<WofCacheItem>) =
            crossbeam::channel::bounded(1024);
        let (to_index_sender, to_index_receiver): (Sender<SchemafiedPoi>, Receiver<SchemafiedPoi>) =
            crossbeam::channel::bounded(1024);
        let mut handles = vec![];

        let admin_cache = self.admin_cache.clone();

        handles.push(spawn_blocking(move || {
            let mut write = admin_cache.begin_write().unwrap();
            let mut count = 0;
            loop {
                count += 1;
                if count % 5000 == 0 {
                    write.commit().unwrap();
                    write = admin_cache.begin_write().unwrap();
                }
                match to_cache_receiver.recv() {
                    Ok(WofCacheItem::Names(admin, names)) => {
                        let mut table = write.open_table(TABLE_NAMES).unwrap();
                        let packed = names.join("\0");
                        table.insert(admin, packed.as_str()).unwrap();
                    }
                    Ok(WofCacheItem::Langs(admin, langs)) => {
                        let mut table = write.open_table(TABLE_LANGS).unwrap();
                        let packed = langs.join("\0");
                        table.insert(admin, packed.as_str()).unwrap();
                    }
                    Ok(WofCacheItem::Admins(s2cell, admins)) => {
                        let mut table = write.open_table(TABLE_AREAS).unwrap();
                        let packed = admins
                            .iter()
                            .flat_map(|id| id.to_le_bytes())
                            .collect::<Vec<_>>();
                        table.insert(s2cell, packed.as_slice()).unwrap();
                    }
                    Err(_) => break,
                }
            }
        }));

        let mut writer = self.index.writer()?;

        handles.push(spawn_blocking(move || {
            let start = std::time::Instant::now();

            let mut count = 0;
            loop {
                {
                    count += 1;
                    if count % 10000 == 0 {
                        info!(
                            "{} POIs parsed in {} seconds, {} per second.",
                            count,
                            start.elapsed().as_secs(),
                            count as f64 / start.elapsed().as_secs_f64(),
                        );
                    }
                }

                if let Ok(poi) = to_index_receiver.recv() {
                    if let Err(err) = writer.add_poi(poi, &source) {
                        warn!("Failed to add POI to index. {}", err);
                    }
                } else {
                    break;
                }
            }
            writer.commit().unwrap();
        }));

        // Spawn processing workers
        for _ in 0..num_cpus::get() {
            let no_admin_receiver = receiver.clone();
            let to_index_sender = to_index_sender.clone();
            let to_cache_sender = to_cache_sender.clone();
            let admin_cache = self.admin_cache.clone();
            let wof_db = self.wof_db.clone();
            let pip_tree = self.pip_tree.clone();

            handles.push(spawn(async move {
                let mut read = admin_cache.begin_read().unwrap();
                let mut counter = 0;
                while let Ok(mut poi) = no_admin_receiver.recv() {
                    counter += 1;
                    if counter % 1000 == 0 {
                        read = admin_cache.begin_read().unwrap();

                        trace!(
                            "Cache queue, index queue: {}, {}",
                            to_cache_sender.len(),
                            to_index_sender.len()
                        );
                    }

                    match populate_admin_areas(
                        &read,
                        to_cache_sender.clone(),
                        &mut poi,
                        &wof_db,
                        &pip_tree,
                    )
                    .await
                    {
                        Ok(()) => {
                            let poi = SchemafiedPoi::from(poi);
                            to_index_sender.send(poi).unwrap();
                        }
                        Err(err) => {
                            warn!("Failed to populate admin areas, {}", err);
                        }
                    }
                }
            }));
        }
        drop(to_index_sender);
        drop(to_cache_sender);

        info!("Waiting for indexing to finish");
        join_all(handles).await;
        info!("Indexing complete");

        Ok(())
    }
}
