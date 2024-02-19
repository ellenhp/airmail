use airmail_common::categories::PoiCategory;
use geojson::{GeoJson, Value};

use super::{
    substitutions::{permute_housenum, permute_road, permute_unit},
    AirmailPoi,
};

pub fn parse_oa_geojson(
    object: &geojson::GeoJson,
) -> Result<AirmailPoi, Box<dyn std::error::Error>> {
    match object {
        GeoJson::Feature(feature) => {
            let properties = feature.properties.as_ref().unwrap();
            let name = vec![];
            let category = PoiCategory::Address;
            let house_number =
                if let Some(house_num) = properties.get("number").map(|v| v.as_str()).flatten() {
                    permute_housenum(house_num)?
                } else {
                    return Err("No house number".into());
                };
            let road = if let Some(road) = properties.get("street").map(|v| v.as_str()).flatten() {
                permute_road(road)?
            } else {
                return Err("No road".into());
            };
            let unit = if let Some(unit) = properties.get("unit").map(|v| v.as_str()).flatten() {
                permute_unit(unit)?
            } else {
                vec![]
            };

            let (lat, lng) = match &feature.geometry {
                Some(geometry) => match &geometry.value {
                    Value::Point(point) => {
                        let lat = point[1];
                        let lng = point[0];
                        (lat, lng)
                    }
                    _ => panic!(),
                },
                None => panic!(),
            };
            Ok(AirmailPoi::new(
                name,
                "openaddresses".to_string(),
                category,
                house_number,
                road,
                unit,
                lat,
                lng,
                vec![], // OpenAddresses doesn't have tags. :(
            )?)
        }
        _ => Err("Not a feature".into()),
    }
}
