use std::{collections::HashMap, error::Error};

use airmail::poi::ToIndexPoi;
use geo::{Centroid, Coord, LineString, Polygon};
use log::{debug, warn};
use osmx::{Database, Locations, Transaction, Way};

fn tags_to_poi(tags: &HashMap<String, String>, lat: f64, lng: f64) -> Option<ToIndexPoi> {
    if tags.is_empty() {
        return None;
    }
    if tags.contains_key("highway")
        || tags.contains_key("natural")
        || tags.contains_key("boundary")
        || tags.contains_key("admin_level")
    {
        return None;
    }

    let house_number = tags.get("addr:housenumber").map(|s| s.to_string());
    let road = tags.get("addr:street").map(|s| s.to_string());
    let unit = tags.get("addr:unit").map(|s| s.to_string());

    let names = {
        let mut names = Vec::new();
        tags.iter()
            .filter(|(key, _value)| key.contains("name:") || key.to_string() == "name")
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
            tags.into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .unwrap(),
    )
}

fn way_centroid(way: &Way, location_table: &Locations) -> Option<(f64, f64)> {
    let node_positions: Vec<Coord> = way
        .nodes()
        .filter_map(|node| {
            let node = location_table.get(node).expect("Nodes must have locations");
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

fn index_way(
    tags: &HashMap<String, String>,
    way: &Way,
    location_table: &Locations,
) -> Option<ToIndexPoi> {
    let (lng, lat) = way_centroid(way, location_table)?;
    tags_to_poi(&tags, lat, lng)
}

fn tags<'a, I: Iterator<Item = (&'a str, &'a str)>>(
    tag_iterator: I,
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut tags = HashMap::new();
    for (key, value) in tag_iterator {
        tags.insert(key.to_string(), value.to_string());
    }
    Ok(tags)
}

pub(crate) fn parse_osm<CB: Sync + Fn(ToIndexPoi) -> Result<(), Box<dyn std::error::Error>>>(
    osmx_path: &str,
    callback: &CB,
) -> Result<(), Box<dyn std::error::Error>> {
    dbg!(osmx_path);
    let db = Database::open(osmx_path).unwrap();
    println!("Processing nodes");
    {
        let osm = Transaction::begin(&db)?;
        let locations = osm.locations()?;
        osm.nodes()?.iter().for_each(|(node_id, node)| {
            let tags = tags(node.tags());
            if let Ok(tags) = tags {
                let location = locations.get(node_id).expect("Nodes must have locations");
                if let Some(poi) = tags_to_poi(&tags, location.lat(), location.lon()) {
                    if let Err(err) = callback(poi) {
                        warn!("Error from callback: {}", err);
                    }
                }
            }
        });
    }
    println!("Processing ways");
    {
        let osm = Transaction::begin(&db)?;
        let locations = osm.locations()?;
        osm.ways()?.iter().for_each(|(_way_id, way)| {
            let tags = tags(way.tags());
            if let Ok(tags) = tags {
                index_way(&tags, &way, &locations).map(|poi| {
                    if let Err(err) = callback(poi) {
                        warn!("Error from callback: {}", err);
                    }
                });
            }
        });
    }
    println!("Skipping relations (FIXME)");
    println!("Done, waiting for worker threads to finish.");
    Ok(())
}
