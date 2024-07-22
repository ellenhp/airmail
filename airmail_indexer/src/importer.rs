use airmail::{
    index::AirmailIndex,
    poi::{SchemafiedPoi, ToIndexPoi},
};
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
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
    admin_cache: String,
    wof_db_path: PathBuf,
}

impl ImporterBuilder {
    pub fn new(whosonfirst_spatialite_path: &Path) -> Self {
        let tmp_dir = std::env::temp_dir();
        let admin_cache = tmp_dir.join("admin_cache.db").to_string_lossy().to_string();

        Self {
            admin_cache,
            wof_db_path: whosonfirst_spatialite_path.to_path_buf(),
        }
    }

    pub fn admin_cache(mut self, admin_cache: &str) -> Self {
        self.admin_cache = admin_cache.to_string();
        self
    }

    pub async fn build(self) -> Result<Importer> {
        let db = Database::create(&self.admin_cache)
            .expect("Failed to open or create administrative area cache database.");
        {
            let txn = db.begin_write().unwrap();
            {
                txn.open_table(TABLE_AREAS).unwrap();
                txn.open_table(TABLE_NAMES).unwrap();
            }
            txn.commit().unwrap();
        }

        let wof_db =
            WhosOnFirst::new(&self.wof_db_path).expect("Failed to open WhosOnFirst database.");

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
        index: &mut AirmailIndex,
        source: &str,
        receiver: Receiver<ToIndexPoi>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut nonblocking_join_handles = Vec::new();
        let (to_cache_sender, to_cache_receiver): (Sender<WofCacheItem>, Receiver<WofCacheItem>) =
            crossbeam::channel::bounded(1024 * 64);
        let (to_index_sender, to_index_receiver): (Sender<SchemafiedPoi>, Receiver<SchemafiedPoi>) =
            crossbeam::channel::bounded(1024 * 64);
        {
            let admin_cache = self.admin_cache.clone();
            std::thread::spawn(move || {
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
        }

        // Our tasks aren't CPU-bound, so we can spawn a few more than the number of CPUs
        // to keep the CPU busy while waiting for IO.
        for worker in 1..num_cpus::get() * 4 {
            println!("Spawning worker {}", worker);
            let no_admin_receiver = receiver.clone();
            let to_index_sender = to_index_sender.clone();
            let to_cache_sender = to_cache_sender.clone();
            let admin_cache = self.admin_cache.clone();
            let wof_db = self.wof_db.clone();

            nonblocking_join_handles.push(spawn(async move {
                let mut read = admin_cache.begin_read().unwrap();
                let mut counter = 0;
                while let Ok(mut poi) = no_admin_receiver.recv() {
                    counter += 1;
                    if counter % 1000 == 0 {
                        read = admin_cache.begin_read().unwrap();
                    }
                    let mut sent = false;
                    for attempt in 0..5 {
                        if attempt > 0 {
                            println!("Retrying to populate admin areas.");
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        }

                        if let Err(err) =
                            populate_admin_areas(&read, to_cache_sender.clone(), &mut poi, &wof_db)
                                .await
                        {
                            println!(
                                "Failed to populate admin areas, {}, attempt: {}",
                                err, attempt
                            );
                        } else {
                            let poi = SchemafiedPoi::from(poi);
                            to_index_sender.send(poi).unwrap();
                            sent = true;
                            break;
                        }
                    }
                    if !sent {
                        println!("Failed to populate admin areas after 5 attempts. Skipping POI.");
                    }
                }
            }));
        }
        drop(to_index_sender);
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
                if let Err(err) = writer.add_poi(poi, source).await {
                    println!("Failed to add POI to index. {}", err);
                }
            } else {
                break;
            }
        }
        writer.commit().unwrap();

        println!("Waiting for tasks to finish.");
        for handle in nonblocking_join_handles {
            handle.await.unwrap();
        }

        Ok(())
    }
}
