use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use airmail::poi::ToIndexPoi;
use anyhow::Result;
use crossbeam::channel::Sender;
use log::{info, warn};
use osmpbf::{Element, ElementReader};

use crate::openstreetmap::OsmPoi;

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

    pub fn parse_osm(self) -> Result<()> {
        info!("Parsing OSM PBF file: {}", self.osm_pbf.display());
        let reader = ElementReader::from_path(&self.osm_pbf)?;
        let count_ways = AtomicUsize::new(0);
        let count_nodes = AtomicUsize::new(0);
        let count_dense_nodes = AtomicUsize::new(0);

        // Extract points of interest
        let pois = reader.par_map_reduce(
            |element| match element {
                // A dense node is a point.
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
                // A node is a point.
                // Depending on the format, unlikely to contain any interesting information, likely
                // where the actual geometry is contained for a way.
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
                // Depending on the format, the actual geometry might be contained
                // in the nodes and so isn't of interest
                Element::Way(way) => {
                    let location = way
                        .node_locations()
                        .next()
                        .map(|location| (location.lat(), location.lon()));
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
