use std::collections::HashMap;

use airmail::poi::ToIndexPoi;

#[allow(clippy::module_name_repetitions)]
pub struct OsmPoi {
    tags: HashMap<String, String>,
    location: (f64, f64),
}

impl OsmPoi {
    /// Create a new `OsmPoi` from a set of tags and a location.
    pub fn new(tags: HashMap<&str, &str>, location: (f64, f64)) -> Option<Self> {
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

        Some(Self { tags, location })
    }

    pub fn index_poi(self) -> Option<ToIndexPoi> {
        self.into()
    }
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
