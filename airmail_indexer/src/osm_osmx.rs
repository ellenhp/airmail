use std::collections::HashMap;

use airmail::poi::ToIndexPoi;
use airmail_indexer::error::IndexerError;
use anyhow::Result;
use crossbeam::channel::Sender;
use geo::{Centroid, Coord, LineString, Polygon};
use log::{debug, info, warn};
use osmx::{Database, Locations, Transaction, Way};

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

    /// Way geometry approach - Get the centroid of the way
    ///
    /// This requires all nodes to be fetched, then all locations to be
    /// resolved from sqlite. The centroid is useful for building locations (closed line strings) and
    /// other POIs, but for roads and other linear features it will be off the line.
    fn way_centroid(way: &Way, locations: &Locations) -> Option<(f64, f64)> {
        // Fetch all nodes, driving the iterator to completion at once
        let positions = way.nodes().collect::<Vec<_>>();

        // Lookup each position
        let node_positions: Vec<Coord> = positions
            .iter()
            .filter_map(|node| {
                let node = locations.get(*node)?;
                Some(Coord::from((node.lon(), node.lat())))
            })
            .collect();

        if node_positions.is_empty() {
            debug!("Empty node_positions");
        }
        let linestring = LineString::new(node_positions);
        let polygon = Polygon::new(linestring, vec![]);
        let centroid = polygon.centroid();
        if centroid.is_none() {
            debug!("No centroid for way");
        }
        let centroid = centroid?;
        Some((centroid.x(), centroid.y()))
    }

    // /// As an extension of way_centroid, linear features will ideally have a mid point
    // /// on the line. This is a faster approach than the centroid, but still requires
    // /// all nodes to be fetched, but only one node to resolve.
    // fn mid_point_on_way(way: &Way, locations: &Locations) -> Option<(f64, f64)> {
    //     // Fetch all nodes, driving the iterator to completion at once
    //     let positions = way.nodes().collect::<Vec<_>>();

    //     // Find the mid point, on the line. So the position will be on the line,
    //     // which might be off the line somewhere.
    //     let mid_position = positions.get(positions.len() / 2)?;

    //     // Lookup each position
    //     let location = locations.get(*mid_position)?;

    //     Some((location.lat(), location.lon()))
    // }

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

                if let Some(interesting_poi) = OsmPoi::new(tags, (location.lat(), location.lon())) {
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

                // Retrieving/iterating the tags is costly, so we only do it if we have a location
                if let Some(location) = Self::way_centroid(&way, &locations) {
                    let tags = way.tags().collect::<HashMap<_, _>>();
                    if let Some(interesting_poi) = OsmPoi::new(tags, location) {
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
