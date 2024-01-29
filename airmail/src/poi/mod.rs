use std::error::Error;

use airmail_common::categories::PoiCategory;
use s2::{cellid::CellID, latlng::LatLng};

pub mod openaddresses;
pub mod query_pip;
pub mod substitutions;

#[derive(Debug, Clone)]
pub struct AirmailPoi {
    pub name: Vec<String>,
    pub category: Vec<String>,
    pub house_number: Vec<String>,
    pub road: Vec<String>,
    pub unit: Vec<String>,
    pub locality: Vec<String>,
    pub region: Vec<String>,
    pub country: Vec<String>,
    pub s2cell: u64,
}

impl AirmailPoi {
    pub fn new(
        name: Vec<String>,
        category: Vec<PoiCategory>,
        house_number: Vec<String>,
        road: Vec<String>,
        unit: Vec<String>,
        lat: f64,
        lng: f64,
    ) -> Result<Self, Box<dyn Error>> {
        let s2cell = s2::cellid::CellID::from(s2::latlng::LatLng::from_degrees(lat, lng)).0;

        Ok(Self {
            name,
            category: vec![], // FIXME.
            house_number,
            road,
            unit,
            locality: Vec::new(),
            region: Vec::new(),
            country: Vec::new(),
            s2cell,
        })
    }

    pub async fn populate_admin_areas(&mut self) -> Result<(), Box<dyn Error>> {
        let cell = CellID(self.s2cell);
        let latlng = LatLng::from(cell);
        let pip_response = query_pip::query_pip(latlng.lat.deg(), latlng.lng.deg()).await?;
        let locality = pip_response
            .locality
            .unwrap_or_default()
            .iter()
            .map(|a| a.name.to_lowercase())
            .collect();
        let region = pip_response
            .region
            .unwrap_or_default()
            .iter()
            .map(|a| a.name.to_lowercase())
            .collect();
        let country = pip_response
            .country
            .unwrap_or_default()
            .iter()
            .map(|a| a.name.to_lowercase())
            .collect();

        self.locality = locality;
        self.region = region;
        self.country = country;

        Ok(())
    }
}
