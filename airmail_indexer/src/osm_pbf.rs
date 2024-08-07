use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use airmail::poi::ToIndexPoi;
use airmail_indexer::cache::{IndexerCache, WofCacheItem};
use anyhow::Result;
use clap::ValueEnum;
use crossbeam::channel::Sender;
use log::{info, warn};
use osmpbf::{Element, ElementReader};

use crate::osm::OsmPoi;

/// An OpenStreetMap PBF file loader.
///
/// OSM PBF contains nodes, ways and relations. This loader extracts points of interest from
/// nodes and ways. The location of a node or way may be present in the data, or
/// may require a lookup from other nodes. To prevent a full scan, the location of all nodes
/// is cached.
pub struct OsmPbf {
    pbf_path: PathBuf,
    nodes_already_cached: bool,
    ignore: Vec<ParseOsmTypes>,
    sender: Sender<ToIndexPoi>,
    indexer_cache: Arc<IndexerCache>,
}

impl OsmPbf {
    pub fn new(
        osm_pbf_path: &Path,
        nodes_already_cached: bool,
        ignore: Vec<ParseOsmTypes>,
        sender: Sender<ToIndexPoi>,
        indexer_cache: Arc<IndexerCache>,
    ) -> Self {
        Self {
            pbf_path: osm_pbf_path.to_path_buf(),
            nodes_already_cached,
            ignore,
            sender,
            indexer_cache,
        }
    }

    pub fn parse_osm(self) -> Result<()> {
        // This call is very expensive as everything has to be read from the PBF file,
        // decompressing and parsing.
        if !self.nodes_already_cached {
            info!("Generating OSM node map from: {}", self.pbf_path.display());
            self.cache_nodes_for_ways()?;
        }

        let count_ways = AtomicUsize::new(0);
        let count_nodes = AtomicUsize::new(0);
        let count_dense_nodes = AtomicUsize::new(0);

        info!("Parsing POIs");

        // Extract points of interest. Par_map_reduce is used to paralleliase the extraction of
        // POIs from the OSM PBF file.
        let pois = ElementReader::from_path(&self.pbf_path)?.par_map_reduce(
            |element| match element {
                // A dense node is a POI (with tags), and has the location embedded in the data.
                Element::DenseNode(dn) => {
                    if self.ignore.contains(&ParseOsmTypes::DenseNodes) {
                        return 0;
                    }
                    let tags = dn.tags().collect::<HashMap<_, _>>();

                    if let Some(interesting_poi) = OsmPoi::new_from_node(tags, (dn.lat(), dn.lon()))
                        .and_then(OsmPoi::index_poi)
                    {
                        count_dense_nodes.fetch_add(1, Ordering::Relaxed);
                        self.sender.send(interesting_poi).expect("sender failed");
                        1
                    } else {
                        0
                    }
                }

                // A node maps something by ID to a location, without tags.
                Element::Node(node) => {
                    if self.ignore.contains(&ParseOsmTypes::Nodes) {
                        return 0;
                    }
                    let tags = node.tags().collect::<HashMap<_, _>>();

                    if let Some(interesting_poi) =
                        OsmPoi::new_from_node(tags, (node.lat(), node.lon()))
                            .and_then(OsmPoi::index_poi)
                    {
                        count_nodes.fetch_add(1, Ordering::Relaxed);
                        self.sender.send(interesting_poi).expect("sender failed");
                        1
                    } else {
                        0
                    }
                }

                // A way is a polyline or polygon, like a road.
                Element::Way(way) => {
                    if self.ignore.contains(&ParseOsmTypes::Ways) {
                        return 0;
                    }

                    // Attempt to get the location from the way from the underlying way data,
                    // this requires the way is stored with the option LocationsOnWays enabled.
                    let mut way_points = way
                        .node_locations()
                        .map(|n| (n.lat(), n.lon()))
                        .collect::<Vec<(f64, f64)>>();

                    // If the location is not present in the way data, attempt to get the location
                    // from the node_map previously built.
                    if way_points.is_empty() {
                        way_points = way
                            .refs()
                            .filter_map(|node_id| {
                                self.indexer_cache
                                    .query_node_location(node_id)
                                    .ok()
                                    .flatten()
                            })
                            .collect::<Vec<(f64, f64)>>();
                    }

                    // Clippy doesn't seem to realise way_points may have been populated
                    #[allow(clippy::if_not_else)]
                    if !way_points.is_empty() {
                        let tags = way.tags().collect::<HashMap<_, _>>();
                        if let Some(interesting_poi) =
                            OsmPoi::new_from_way(tags, &way_points).and_then(OsmPoi::index_poi)
                        {
                            count_ways.fetch_add(1, Ordering::Relaxed);
                            self.sender.send(interesting_poi).expect("sender failed");
                            1
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                }

                // Ignored
                Element::Relation(_) => 0,
            },
            || 0_u64,
            |a, b| a + b,
        )?;

        let count_ways = count_ways.load(Ordering::Relaxed);
        let count_nodes = count_nodes.load(Ordering::Relaxed);
        let count_dense_nodes = count_dense_nodes.load(Ordering::Relaxed);

        info!(
            "Loaded {} interesting pois, made up of {} dense nodes, {} nodes, and {} ways",
            pois, count_dense_nodes, count_nodes, count_ways
        );

        if count_ways == 0 {
            warn!("No ways found in OSM PBF file. Ensure your pbf file has locations present, see Google: LocationsOnWays");
        }

        Ok(())
    }

    fn cache_nodes_for_ways(&self) -> Result<()> {
        // Increase buffer to reduce writes to disk
        self.indexer_cache.buffer_size(10_000_000)?;

        // Store a map of nodes to their locations for quick lookups in the second pass.
        let cached_count = ElementReader::from_path(&self.pbf_path)?.par_map_reduce(
            |element| match element {
                Element::Node(node) => {
                    let location = (node.lat(), node.lon());
                    let _ = self
                        .indexer_cache
                        .buffered_write_item(WofCacheItem::NodeLocation(node.id(), location))
                        .map_err(|e| {
                            warn!("Error writing node location to cache: {}", e);
                        });
                    1
                }
                Element::DenseNode(node) => {
                    let location = (node.lat(), node.lon());
                    let _ = self
                        .indexer_cache
                        .buffered_write_item(WofCacheItem::NodeLocation(node.id(), location))
                        .map_err(|e| {
                            warn!("Error writing node location to cache: {}", e);
                        });
                    1
                }
                _ => 0,
            },
            || 0_u64,
            |a, b| a + b,
        )?;

        // Revert buffer size to default
        self.indexer_cache.buffer_size_default()?;

        info!("{} node locations are cached", cached_count);

        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone, ValueEnum)]
pub enum ParseOsmTypes {
    Ways,
    Nodes,
    DenseNodes,
}
