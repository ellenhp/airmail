use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use airmail::poi::ToIndexPoi;
use anyhow::Result;
use crossbeam::channel::Sender;
use log::{debug, info, warn};
use osmpbf::{Element, ElementReader};

use crate::openstreetmap::OsmPoi;

/// An OpenStreetMap PBF file loader.
///
/// OSM PBF contains nodes, ways and relations. This loader extracts points of interest from
/// nodes, dense nodes and ways. The location of a node or way may be present in the data, or
/// may require a lookup from a node map.

pub struct OsmPbf {
    osm_pbf: PathBuf,
    sender: Sender<ToIndexPoi>,
}

impl OsmPbf {
    pub fn new(osm_pbf: &Path, sender: Sender<ToIndexPoi>) -> Self {
        Self {
            osm_pbf: osm_pbf.to_path_buf(),
            sender,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn parse_osm(self) -> Result<()> {
        info!("Generating OSM node map from: {}", self.osm_pbf.display());

        // Create a Hashmap of nodes to lat/lon, enabling quick lookup of node locations,
        // as ways may not contain the location data.
        let node_map: Vec<(i64, (f64, f64))> = ElementReader::from_path(&self.osm_pbf)?
            .par_map_reduce(
                |element| match element {
                    Element::Node(node) => {
                        let location = (node.lat(), node.lon());
                        vec![(node.id(), location)]
                    }
                    Element::DenseNode(node) => {
                        let location = (node.lat(), node.lon());
                        vec![(node.id(), location)]
                    }
                    _ => Vec::new(),
                },
                Vec::new,
                |mut a, b| {
                    if b.is_empty() {
                        return a;
                    }
                    a.extend(b);
                    a
                },
            )?;

        let node_map: HashMap<i64, (f64, f64)> = node_map.into_iter().collect();
        let count_ways = AtomicUsize::new(0);
        let count_nodes = AtomicUsize::new(0);
        let count_dense_nodes = AtomicUsize::new(0);

        debug!("Parsing POIs");

        // Extract points of interest. Par_map_reduce is used to paralleliase the extraction of
        // POIs from the OSM PBF file.
        let pois = ElementReader::from_path(&self.osm_pbf)?.par_map_reduce(
            |element| match element {
                // A dense node is a POI (with tags), and has the location embedded in the data.
                Element::DenseNode(dn) => {
                    let tags = dn
                        .tags()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<HashMap<_, _>>();

                    if let Some(interesting_poi) =
                        OsmPoi::new(tags, (dn.lat(), dn.lon())).and_then(OsmPoi::index_poi)
                    {
                        count_dense_nodes.fetch_add(1, Ordering::Relaxed);
                        vec![interesting_poi]
                    } else {
                        vec![]
                    }
                }
                // A node is unlikely to be a POI (with tags and a location), but is instead the
                // location.
                Element::Node(node) => {
                    let tags = node
                        .tags()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<HashMap<_, _>>();
                    if let Some(interesting_poi) =
                        OsmPoi::new(tags, (node.lat(), node.lon())).and_then(OsmPoi::index_poi)
                    {
                        count_nodes.fetch_add(1, Ordering::Relaxed);
                        vec![interesting_poi]
                    } else {
                        vec![]
                    }
                }
                // A way is a polyline or polygon.
                Element::Way(way) => {
                    // Attempt to get the location from the way from the underlying way data,
                    // this requires the way is stored with the option LocationsOnWays enabled.
                    let mut location = way
                        .node_locations()
                        .next()
                        .map(|location| (location.lat(), location.lon()));

                    // If the location is not present in the way data, attempt to get the location
                    // from the node_map we previously built.
                    if location.is_none() {
                        location = way
                            .refs()
                            .next()
                            .and_then(|node_id| node_map.get(&node_id).copied());
                    }

                    if let Some(location) = location {
                        let tags = way
                            .tags()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect::<HashMap<_, _>>();
                        if let Some(interesting_poi) =
                            OsmPoi::new(tags, location).and_then(OsmPoi::index_poi)
                        {
                            count_ways.fetch_add(1, Ordering::Relaxed);
                            vec![interesting_poi]
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                }

                // Ignored
                Element::Relation(_) => vec![],
            },
            Vec::new,
            |mut a, mut b| {
                if b.is_empty() {
                    return a;
                }
                a.append(&mut b);
                a
            },
        )?;

        // Reduce some memory usage
        drop(node_map);

        let count_ways = count_ways.load(Ordering::Relaxed);
        let count_nodes = count_nodes.load(Ordering::Relaxed);
        let count_dense_nodes = count_dense_nodes.load(Ordering::Relaxed);

        info!(
            "Found {} pois, made up of {} dense nodes, {} nodes and {} ways",
            pois.len(),
            count_dense_nodes,
            count_nodes,
            count_ways
        );

        for poi in pois {
            self.sender.send(poi)?;
        }

        if count_ways == 0 {
            warn!("No ways found in OSM PBF file. Ensure your pbf file has locations present, see Google: LocationsOnWays");
        }

        Ok(())
    }
}
