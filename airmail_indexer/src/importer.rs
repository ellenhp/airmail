use airmail::{
    index::AirmailIndex,
    poi::{SchemafiedPoi, ToIndexPoi},
};
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use log::debug;
use redb::Database;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::spawn;

use crate::{
    populate_admin_areas, wof::WhosOnFirst, WofCacheItem, TABLE_AREAS, TABLE_LANGS, TABLE_NAMES,
};

pub struct ImporterBuilder {
    admin_cache: Option<PathBuf>,
    wof_db_path: PathBuf,
}

impl ImporterBuilder {
    pub fn new(whosonfirst_spatialite_path: &Path) -> Self {
        Self {
            admin_cache: None,
            wof_db_path: whosonfirst_spatialite_path.to_path_buf(),
        }
    }

    pub fn admin_cache(mut self, admin_cache: &Path) -> Self {
        self.admin_cache = Some(admin_cache.to_path_buf());
        self
    }

    pub async fn build(self) -> Result<Importer> {
        let admin_cache = if let Some(admin_cache) = self.admin_cache {
            admin_cache
        } else {
            std::env::temp_dir().join("admin_cache.db")
        };

        let db = Database::create(&admin_cache)?;
        {
            let txn = db.begin_write().unwrap();
            {
                txn.open_table(TABLE_AREAS).unwrap();
                txn.open_table(TABLE_NAMES).unwrap();
            }
            txn.commit().unwrap();
        }

        let wof_db = WhosOnFirst::new(&self.wof_db_path).await?;

        Ok(Importer {
            admin_cache: Arc::new(db),
            wof_db,
        })
    }
}

pub struct Importer {
    admin_cache: Arc<Database>,
    wof_db: WhosOnFirst,
}

impl Importer {
    pub async fn run_import(
        &self,
        mut index: AirmailIndex,
        source: &str,
        receiver: Receiver<ToIndexPoi>,
    ) -> Result<()> {
        let source = source.to_string();
        // let mut nonblocking_join_handles = Vec::new();
        let (to_cache_sender, to_cache_receiver): (Sender<WofCacheItem>, Receiver<WofCacheItem>) =
            crossbeam::channel::bounded(1024);
        let (to_index_sender, to_index_receiver): (Sender<SchemafiedPoi>, Receiver<SchemafiedPoi>) =
            crossbeam::channel::bounded(1024);

        let admin_cache = self.admin_cache.clone();
        let cache = spawn(async move {
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
        });

        let index_builder = spawn(async move {
            let start = std::time::Instant::now();

            let mut writer = index.writer().unwrap();
            let mut count = 0;
            loop {
                {
                    count += 1;
                    if count % 10000 == 0 {
                        println!(
                            "{} POIs parsed in {} seconds, {} per second.",
                            count,
                            start.elapsed().as_secs(),
                            count as f64 / start.elapsed().as_secs_f64(),
                        );
                    }
                }

                if let Ok(poi) = to_index_receiver.recv() {
                    if let Err(err) = writer.add_poi(poi, &source).await {
                        println!("Failed to add POI to index. {}", err);
                    }
                } else {
                    break;
                }
            }
            writer.commit().unwrap();
        });

        for _ in 0..num_cpus::get_physical() {
            let no_admin_receiver = receiver.clone();
            let to_index_sender = to_index_sender.clone();
            let to_cache_sender = to_cache_sender.clone();
            let admin_cache = self.admin_cache.clone();
            let wof_db = self.wof_db.clone();

            // nonblocking_join_handles.push(spawn(async move {
            let mut read = admin_cache.begin_read().unwrap();
            let mut counter = 0;
            while let Ok(mut poi) = no_admin_receiver.recv() {
                counter += 1;
                if counter % 1000 == 0 {
                    read = admin_cache.begin_read().unwrap();

                    debug!(
                        "Cache queue, index queue: {}, {}",
                        to_cache_sender.len(),
                        to_index_sender.len()
                    );
                }

                match populate_admin_areas(&read, to_cache_sender.clone(), &mut poi, &wof_db).await
                {
                    Ok(()) => {
                        let poi = SchemafiedPoi::from(poi);
                        to_index_sender.send(poi).unwrap();
                    }
                    Err(err) => {
                        println!("Failed to populate admin areas, {}", err);
                    }
                }
            }
            // }));
        }
        drop(to_index_sender);

        println!("Waiting for tasks to finish.");
        cache.await.unwrap();
        index_builder.await.unwrap();

        Ok(())
    }
}
