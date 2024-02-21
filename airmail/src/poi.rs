use std::error::Error;

use airmail_common::categories::PoiCategory;
use serde::{Deserialize, Serialize};

pub struct ToIndexPoi {
    pub content: Vec<String>,
    pub source: String,
    pub s2cell: u64,
    pub tags: Vec<(String, String)>,
}

impl From<AirmailPoi> for ToIndexPoi {
    fn from(poi: AirmailPoi) -> Self {
        let mut content = Vec::new();
        content.extend(poi.name);
        content.extend(poi.house_number);
        content.extend(poi.road);
        content.extend(poi.unit);
        content.extend(poi.admins);
        content.extend(poi.category.labels());

        Self {
            content,
            source: poi.source,
            s2cell: poi.s2cell,
            tags: poi.tags,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirmailPoi {
    pub name: Vec<String>,
    pub source: String,
    pub category: PoiCategory,
    pub house_number: Vec<String>,
    pub road: Vec<String>,
    pub unit: Vec<String>,
    pub admins: Vec<String>,
    pub s2cell: u64,
    pub lat: f64,
    pub lng: f64,
    pub tags: Vec<(String, String)>,
}

impl AirmailPoi {
    pub fn new(
        name: Vec<String>,
        source: String,
        category: PoiCategory,
        house_number: Vec<String>,
        road: Vec<String>,
        unit: Vec<String>,
        lat: f64,
        lng: f64,
        tags: Vec<(String, String)>,
    ) -> Result<Self, Box<dyn Error>> {
        let s2cell = s2::cellid::CellID::from(s2::latlng::LatLng::from_degrees(lat, lng)).0;

        Ok(Self {
            name,
            source,
            category,
            house_number,
            road,
            unit,
            admins: Vec::new(),
            s2cell,
            lat,
            lng,
            tags,
        })
    }
}
