use std::collections::HashMap;

use airmail::poi::ToIndexPoi;
use airmail_indexer::error::IndexerError;
use anyhow::Result;
use crossbeam::channel::Sender;
use log::{debug, info, warn};
use osmx::{Database, Locations, Transaction};

use crate::osm::OsmPoi;

pub struct OSMExpressLoader<'db> {
    sender: Sender<ToIndexPoi>,
    transaction: Transaction<'db>,
}

impl<'db> OSMExpressLoader<'db> {
    pub fn new(db: &'db Database, sender: Sender<ToIndexPoi>) -> Result<Self> {
        // Share the transaction within the loader
        let transaction = Transaction::begin(db).map_err(IndexerError::from)?;

        Ok(Self {
            sender,
            transaction,
        })
    }

    fn locations(&self) -> Result<Locations> {
        let locations = self.transaction.locations().map_err(IndexerError::from)?;
        Ok(locations)
    }

    /// Parse an `OSMExpress` file and send POIs for indexing.
    pub(crate) fn parse_osm(self) -> Result<()> {
        let mut total = 0;
        let mut interesting = 0;
        let locations = self.locations()?;

        info!("Loading OSM nodes");
        {
            for (node_id, node) in self.transaction.nodes().map_err(IndexerError::from)?.iter() {
                total += 1;
                if interesting % 10000 == 0 {
                    debug!(
                        "Loaded OSM nodes interesting/total: {}/{} nodes, queue size: {}",
                        interesting,
                        total,
                        self.sender.len()
                    );
                }

                let location = locations
                    .get(node_id)
                    .ok_or(IndexerError::NodeMissingLocation)?;

                let tags = node.tags().collect::<HashMap<_, _>>();

                if let Some(interesting_poi) =
                    OsmPoi::new_from_node(tags, (location.lat(), location.lon()))
                {
                    if let Some(poi_to_indexer) = interesting_poi.into() {
                        self.sender.send(poi_to_indexer).map_err(|e| {
                            warn!("Error from sender: {}", e);
                            e
                        })?;
                        interesting += 1;
                    }
                }
            }
        }

        info!("Loading OSM ways");
        {
            for (_way_id, way) in self.transaction.ways().map_err(IndexerError::from)?.iter() {
                if interesting % 10000 == 0 {
                    debug!(
                        "Loaded OSM ways interesting/total: {}/{} nodes, queue size: {}",
                        interesting,
                        total,
                        self.sender.len()
                    );
                }

                // This requires all nodes to be fetched, then all locations to be
                // resolved from sqlite.
                let nodes = way.nodes().collect::<Vec<_>>();

                // Lookup each position
                let way_points = nodes
                    .iter()
                    .filter_map(|node| {
                        let node = locations.get(*node)?;
                        Some((node.lat(), node.lon()))
                    })
                    .collect::<Vec<(f64, f64)>>();

                // Retrieving/iterating the tags is costly, so we only do it if we have a location
                if !way_points.is_empty() {
                    let tags = way.tags().collect::<HashMap<_, _>>();
                    if let Some(interesting_poi) = OsmPoi::new_from_way(tags, &way_points) {
                        if let Some(poi_to_indexer) = interesting_poi.into() {
                            self.sender.send(poi_to_indexer).map_err(|e| {
                                warn!("Error from sender: {}", e);
                                e
                            })?;
                            interesting += 1;
                        }
                    }
                }
            }
        }

        info!("Skipping relations (FIXME)");
        info!("OSM parsing complete");
        Ok(())
    }
}
