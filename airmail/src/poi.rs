use std::error::Error;

use airmail_common::categories::PoiCategory;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirmailPoi {
    pub name: Vec<String>,
    pub source: String,
    pub category: Vec<String>,
    pub house_number: Vec<String>,
    pub road: Vec<String>,
    pub unit: Vec<String>,
    pub locality: Vec<String>,
    pub region: Vec<String>,
    pub country: Vec<String>,
    pub s2cell: u64,
    pub lat: f64,
    pub lng: f64,
    pub tags: Vec<(String, String)>,
}

impl AirmailPoi {
    pub fn new(
        name: Vec<String>,
        source: String,
        category: Vec<PoiCategory>,
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
            category: category
                .iter()
                .map(|category| category.to_facet())
                .collect(), // FIXME.
            house_number,
            road,
            unit,
            locality: Vec::new(),
            region: Vec::new(),
            country: Vec::new(),
            s2cell,
            lat,
            lng,
            tags,
        })
    }
}
