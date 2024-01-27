use geojson::{GeoJson, Value};

fn sanitize_oa_field(field: Option<&str>) -> Option<String> {
    field.map(|field| {
        let field = field.to_lowercase();
        let parts: Vec<_> = field.split_whitespace().collect();
        parts.join(" ")
    })
}
pub struct AirmailPoi {
    pub name: Option<String>,
    pub category: Option<String>,
    pub house_number: Option<String>,
    pub road: Option<String>,
    pub unit: Option<String>,
    pub locality: Option<String>,
    pub region: Option<String>,
    pub s2cell: u64,
}

impl AirmailPoi {
    pub fn from_openaddresses_geojson(
        object: &geojson::GeoJson,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        match object {
            GeoJson::Feature(feature) => {
                let properties = feature.properties.as_ref().unwrap();
                let name = None;
                let category = None;
                let house_number =
                    sanitize_oa_field(properties.get("number").map(|v| v.as_str()).flatten());
                let road =
                    sanitize_oa_field(properties.get("street").map(|v| v.as_str()).flatten());
                let unit = sanitize_oa_field(properties.get("unit").map(|v| v.as_str()).flatten());
                let locality =
                    sanitize_oa_field(properties.get("city").map(|v| v.as_str()).flatten());

                let s2cell = match &feature.geometry {
                    Some(geometry) => match &geometry.value {
                        Value::Point(point) => {
                            let lat = point[1];
                            let lng = point[0];
                            let s2cell: s2::cellid::CellID =
                                s2::latlng::LatLng::from_degrees(lat, lng).into();
                            s2cell
                        }
                        _ => panic!(),
                    },
                    None => panic!(),
                };
                Ok(Self {
                    name,
                    category,
                    house_number,
                    road,
                    unit,
                    locality,
                    region: None,
                    s2cell: s2cell.0,
                })
            }
            _ => Err("Not a feature".into()),
        }
    }
}
