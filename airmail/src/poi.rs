use anyhow::Result;
use lingua::Language;
use serde::{Deserialize, Serialize};

use crate::substitutions::permute_road;

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

pub struct SchemafiedPoi {
    pub content: Vec<String>,
    pub s2cell: u64,
    pub s2cell_parents: Vec<u64>,
    pub tags: Vec<(String, String)>,
}

impl From<ToIndexPoi> for SchemafiedPoi {
    fn from(poi: ToIndexPoi) -> Self {
        let mut content = Vec::new();
        content.extend(poi.names);
        content.extend(poi.house_number);
        if let Some(road) = poi.road {
            for lang in poi.languages {
                content.extend(permute_road(&road, &lang).expect("Failed to permute road"));
            }
        }
        content.extend(poi.unit);
        content.extend(poi.admins);

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
