use airmail::{
    index::AirmailIndex,
    poi::{SchemafiedPoi, ToIndexPoi},
};
use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use futures_util::future::join_all;
use lingua::{IsoCode639_3, Language};
use log::{info, trace, warn};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
use tokio::{
    spawn,
    task::{spawn_blocking, JoinHandle},
};

use crate::{
    cache::{IndexerCache, WofCacheItem},
    pip_tree::PipTree,
    query_pip,
    wof::{ConcisePipResponse, WhosOnFirst},
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
        let admin_cache_path = if let Some(admin_cache) = self.admin_cache_path {
            admin_cache
        } else {
            std::env::temp_dir().join("admin_cache.db")
        };

        let admin_cache = IndexerCache::new(&admin_cache_path)?;

        let wof_db = WhosOnFirst::new(&self.wof_db_path).await?;

        let pip_tree = if let Some(pip_tree_cache) = self.pip_tree_path {
            Some(PipTree::new_or_load(&wof_db, &pip_tree_cache).await?)
        } else {
            None
        };

        Importer::new(self.index, admin_cache, wof_db, pip_tree).await
    }
}

pub struct Importer {
    index: AirmailIndex,
    indexer_cache: Arc<IndexerCache>,
    wof_db: WhosOnFirst,
    pip_tree: Option<PipTree<ConcisePipResponse>>,
}

impl Importer {
    pub async fn new(
        index: AirmailIndex,
        indexer_cache: IndexerCache,
        wof_db: WhosOnFirst,
        pip_tree: Option<PipTree<ConcisePipResponse>>,
    ) -> Result<Self> {
        Ok(Self {
            index,
            indexer_cache: Arc::new(indexer_cache),
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
        let mut handles: Vec<JoinHandle<Result<()>>> = vec![];

        // Listen for items to cache
        let admin_cache = self.indexer_cache.clone();
        handles.push(spawn_blocking(move || {
            while let Ok(cache_item) = to_cache_receiver.recv() {
                admin_cache.buffered_write_item(cache_item)?;
            }
            Ok(())
        }));

        // Listen for items to index
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
            writer.commit()?;

            Ok(())
        }));

        // Spawn processing workers
        for _ in 0..num_cpus::get() {
            let no_admin_receiver = receiver.clone();
            let to_index_sender = to_index_sender.clone();
            let to_cache_sender = to_cache_sender.clone();
            let indexer_cache = self.indexer_cache.clone();
            let wof_db = self.wof_db.clone();
            let pip_tree = self.pip_tree.clone();

            handles.push(spawn(async move {
                let mut counter = 0;
                while let Ok(poi) = no_admin_receiver.recv() {
                    counter += 1;
                    if counter % 1000 == 0 {
                        trace!(
                            "Cache queue, index queue: {}, {}",
                            to_cache_sender.len(),
                            to_index_sender.len()
                        );
                    }

                    match Self::populate_admin_areas(
                        poi,
                        &indexer_cache,
                        to_cache_sender.clone(),
                        &wof_db,
                        &pip_tree,
                    )
                    .await
                    {
                        Ok(poi) => {
                            let schemafied_poi = SchemafiedPoi::from(poi);
                            to_index_sender.send(schemafied_poi).unwrap();
                        }
                        Err(err) => {
                            warn!("Failed to populate admin areas, {}", err);
                        }
                    }
                }

                Ok(())
            }));
        }
        drop(to_index_sender);
        drop(to_cache_sender);

        trace!("Waiting for indexing to finish");
        join_all(handles).await;
        info!("Indexing complete");

        Ok(())
    }

    pub fn indexer_cache(&self) -> Arc<IndexerCache> {
        self.indexer_cache.clone()
    }

    async fn populate_admin_areas(
        mut poi: ToIndexPoi,
        indexer_cache: &IndexerCache,
        to_cache_sender: Sender<WofCacheItem>,
        wof_db: &WhosOnFirst,
        pip_tree: &Option<PipTree<ConcisePipResponse>>,
    ) -> Result<ToIndexPoi> {
        let pip_response =
            query_pip::query_pip(indexer_cache, to_cache_sender, poi.s2cell, wof_db, pip_tree)
                .await?;
        for admin in pip_response.admin_names {
            poi.admins.push(admin);
        }
        for lang in pip_response.admin_langs {
            if let Ok(iso) = IsoCode639_3::from_str(&lang) {
                poi.languages.push(Language::from_iso_code_639_3(&iso))
            }
        }

        Ok(poi)
    }
}
