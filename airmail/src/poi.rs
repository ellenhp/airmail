use anyhow::Result;
use lingua::Language;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirmailPoi {
    pub source: String,
    pub admins: Vec<String>,
    pub s2cell: u64,
    pub lat: f64,
    pub lng: f64,
    pub tags: Vec<(String, String)>,
}

impl AirmailPoi {
    pub fn new(source: String, lat: f64, lng: f64, tags: Vec<(String, String)>) -> Result<Self> {
        let s2cell = s2::cellid::CellID::from(s2::latlng::LatLng::from_degrees(lat, lng)).0;

        Ok(Self {
            source,
            admins: Vec::new(),
            s2cell,
            lat,
            lng,
            tags,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ToIndexPoi {
    pub names: Vec<String>,
    pub house_number: Option<String>,
    pub road: Option<String>,
    pub unit: Option<String>,
    pub admins: Vec<String>,
    pub s2cell: u64,
    pub tags: Vec<(String, String)>,
    pub languages: Vec<Language>,
}

impl ToIndexPoi {
    pub fn new(
        names: Vec<String>,
        house_number: Option<String>,
        road: Option<String>,
        unit: Option<String>,
        lat: f64,
        lng: f64,
        tags: Vec<(String, String)>,
    ) -> Result<Self> {
        let s2cell = s2::cellid::CellID::from(s2::latlng::LatLng::from_degrees(lat, lng)).0;

        Ok(Self {
            names,
            house_number,
            road,
            unit,
            admins: Vec::new(),
            s2cell,
            tags,
            languages: Vec::new(),
        })
    }
}

#[derive(Debug)]
pub struct SchemafiedPoi {
    pub content: Vec<String>,
    pub s2cell: u64,
    pub s2cell_parents: Vec<u64>,
    pub tags: Vec<(String, String)>,
}

fn prefix_strings<I: IntoIterator<Item = String>>(prefix: &str, strings: I) -> Vec<String> {
    return strings
        .into_iter()
        .map(|s| format!("{}:{}", prefix, s))
        .collect();
}

impl From<ToIndexPoi> for SchemafiedPoi {
    fn from(poi: ToIndexPoi) -> Self {
        let mut content = Vec::new();
        content.extend(prefix_strings("name", poi.names));
        content.extend(prefix_strings("house_num", poi.house_number));
        if let Some(road) = poi.road {
            for lang in poi.languages {
                content.extend(prefix_strings(
                    &format!("road:{}", lang.iso_code_639_3()),
                    vec![road.clone()],
                ));
            }
        }
        content.extend(prefix_strings("unit", poi.unit));
        content.extend(prefix_strings("admins", poi.admins));

        let indexed_keys = [
            "natural", "amenity", "shop", "leisure", "tourism", "historic", "cuisine",
        ];
        let indexed_key_prefixes = ["diet:"];
        for (key, value) in &poi.tags {
            if indexed_keys.contains(&key.as_str())
                || indexed_key_prefixes
                    .iter()
                    .any(|prefix| key.starts_with(prefix))
            {
                content.extend(prefix_strings(
                    "tag",
                    value.split(";").map(|v| format!("{key}={v}")),
                ));
            }
        }

        let mut s2cell_parents = Vec::new();
        let cell = s2::cellid::CellID(poi.s2cell);
        for level in 0..cell.level() {
            let cell = cell.parent(level);
            s2cell_parents.push(cell.0);
        }

        Self {
            content,
            s2cell: poi.s2cell,
            s2cell_parents,
            tags: poi.tags,
        }
    }
}
