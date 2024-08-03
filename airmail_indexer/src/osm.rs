use airmail::poi::ToIndexPoi;
use geo::{Centroid, Coord, LineString, Polygon};
use log::debug;
use std::collections::HashMap;

#[allow(clippy::module_name_repetitions)]
pub struct OsmPoi {
    tags: HashMap<String, String>,
    location: (f64, f64),
}

impl OsmPoi {
    /// Create a new `OsmPoi` from a node.
    pub fn new_from_node(tags: HashMap<&str, &str>, point: (f64, f64)) -> Option<Self> {
        let tags = Self::validate_tags(tags)?;
        Some(Self {
            tags,
            location: point,
        })
    }

    /// Create a new `OsmPoi` from a way.
    pub fn new_from_way(tags: HashMap<&str, &str>, points: &[(f64, f64)]) -> Option<Self> {
        let tags = Self::validate_tags(tags)?;
        let location = Self::way_centroid(points)?;
        Some(Self { tags, location })
    }

    /// Validate the tags of a point of interest.
    fn validate_tags(tags: HashMap<&str, &str>) -> Option<HashMap<String, String>> {
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

        let tags = tags
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Some(tags)
    }

    pub fn index_poi(self) -> Option<ToIndexPoi> {
        self.into()
    }

    /// Get the centroid of the way
    ///
    /// The centroid is useful for building locations (closed line strings) and
    /// other POIs, but for roads and other linear features it will be off the line.
    fn way_centroid(points: &[(f64, f64)]) -> Option<(f64, f64)> {
        // Lookup each position
        let node_positions: Vec<Coord> = points
            .iter()
            .map(|point| Coord::from((point.0, point.1)))
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
}

impl From<OsmPoi> for Option<ToIndexPoi> {
    fn from(poi: OsmPoi) -> Option<ToIndexPoi> {
        let (lat, lng) = poi.location;
        let house_number = poi.tags.get("addr:housenumber").map(ToString::to_string);
        let road = poi.tags.get("addr:street").map(ToString::to_string);
        let unit = poi.tags.get("addr:unit").map(ToString::to_string);

        let names = {
            let mut names = Vec::new();
            poi.tags
                .iter()
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

        ToIndexPoi::new(
            names,
            house_number,
            road,
            unit,
            lat,
            lng,
            poi.tags.into_iter().collect(),
        )
        .ok()
    }
}
