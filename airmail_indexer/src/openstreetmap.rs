use std::{collections::HashMap, path::Path};

use airmail::poi::ToIndexPoi;
use airmail_indexer::error::IndexerError;
use anyhow::Result;
use crossbeam::channel::Sender;
use geo::{Centroid, Coord, LineString, Polygon};
use log::{debug, info, warn};
use osmx::{Database, Locations, Transaction, Way};

fn tags_to_poi(tags: &HashMap<String, String>, lat: f64, lng: f64) -> Option<ToIndexPoi> {
    let house_number = tags.get("addr:housenumber").map(ToString::to_string);
    let road = tags.get("addr:street").map(ToString::to_string);
    let unit = tags.get("addr:unit").map(ToString::to_string);

    let names = {
        let mut names = Vec::new();
        tags.iter()
            .filter(|(key, _value)| key.contains("name:") || *key == "name")
            .for_each(|(_key, value)| {
                names.push(value.to_string());
                // TODO: Remove once we get stemmers again.
                if value.contains("'s") {
                    names.push(value.replace("'s", ""));
                    names.push(value.replace("'s", "s"));
                }
            });
        names
    };

    if (house_number.is_none() || road.is_none()) && names.is_empty() {
        return None;
    }

    Some(
        ToIndexPoi::new(
            names,
            house_number,
            road,
            unit,
            lat,
            lng,
            tags.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        )
        .unwrap(),
    )
}

fn way_centroid(way: &Way, location_table: &Locations) -> Option<(f64, f64)> {
    let node_positions: Vec<Coord> = way
        .nodes()
        .map(|node| {
            let node = location_table.get(node).expect("Nodes must have locations");
            Coord::from((node.lon(), node.lat()))
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

fn index_way(
    tags: &HashMap<String, String>,
    way: &Way,
    location_table: &Locations,
) -> Option<ToIndexPoi> {
    let (lng, lat) = way_centroid(way, location_table)?;
    tags_to_poi(tags, lat, lng)
}

fn valid_tags(tags: &HashMap<String, String>) -> bool {
    if tags.is_empty() {
        return false;
    }
    if tags.contains_key("highway")
        || tags.contains_key("natural")
        || tags.contains_key("boundary")
        || tags.contains_key("admin_level")
    {
        return false;
    }

    true
}

fn tags<'a, I: Iterator<Item = (&'a str, &'a str)>>(tag_iterator: I) -> HashMap<String, String> {
    let mut tags = HashMap::new();
    for (key, value) in tag_iterator {
        tags.insert(key.to_string(), value.to_string());
    }

    tags
}

/// Parse an `OSMExpress` file and send POIs for indexing.
pub(crate) fn parse_osm(osmx_path: &Path, sender: &Sender<ToIndexPoi>) -> Result<()> {
    info!("Loading osmx from path: {:?}", osmx_path);
    let db = Database::open(osmx_path).map_err(IndexerError::from)?;
    let osm = Transaction::begin(&db).map_err(IndexerError::from)?;
    let locations = osm.locations().map_err(IndexerError::from)?;
    let mut interesting = 0;
    let mut total = 0;
    info!("Processing nodes");
    {
        for (node_id, node) in osm.nodes().map_err(IndexerError::from)?.iter() {
            total += 1;
            if interesting % 10000 == 0 {
                debug!(
                    "Processed interesting/total: {}/{} nodes, queue size: {}",
                    interesting,
                    total,
                    sender.len()
                );
            }

            let tags = tags(node.tags());
            if valid_tags(&tags) {
                let location = locations.get(node_id).expect("Nodes must have locations");
                if let Some(poi) = tags_to_poi(&tags, location.lat(), location.lon()) {
                    sender.send(poi).map_err(|e| {
                        warn!("Error from sender: {}", e);
                        e
                    })?;
                    interesting += 1;
                }
            }
        }
    }
    info!("Processing ways");
    {
        for (_way_id, way) in osm.ways().map_err(IndexerError::from)?.iter() {
            if interesting % 10000 == 0 {
                debug!(
                    "Processed interesting/total: {}/{} nodes, queue size: {}",
                    interesting,
                    total,
                    sender.len()
                );
            }
            let tags = tags(way.tags());
            if valid_tags(&tags) {
                if let Some(poi) = index_way(&tags, &way, &locations) {
                    sender.send(poi).map_err(|e| {
                        warn!("Error from sender: {}", e);
                        e
                    })?;
                    interesting += 1;
                }
            }
        }
    }
    info!("Skipping relations (FIXME)");
    info!("OSM parsing complete");
    Ok(())
}
