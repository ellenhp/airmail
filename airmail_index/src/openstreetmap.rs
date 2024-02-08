use std::collections::HashMap;

use airmail::poi::AirmailPoi;
use airmail_common::categories::{
    AmenityPoiCategory, EmergencyPoiCategory, FoodPoiCategory, PoiCategory, ShopPoiCategory,
};
use geo::{Centroid, Coord, LineString, MultiPoint, Point, Polygon};
use log::{debug, warn};
use turbosm::{
    element::{Relation, RelationMember, Way},
    Turbosm,
};

use crate::substitutions::{permute_housenum, permute_road, permute_unit};

fn tags_to_poi(tags: &HashMap<String, String>, lat: f64, lng: f64) -> Option<AirmailPoi> {
    if tags.is_empty() {
        return None;
    }
    if tags.contains_key("highway") {
        return None;
    }

    let category = tags.get("amenity").map(|s| match s.as_str() {
        "fast_food" | "food_court" | "cafe" | "pub" | "restaurant" => {
            PoiCategory::Shop(ShopPoiCategory::Food(FoodPoiCategory::Restaurant(None)))
        }
        "biergarten" | "bar" => {
            PoiCategory::Shop(ShopPoiCategory::Food(FoodPoiCategory::Restaurant(None)))
        }
        "drinking_water" => PoiCategory::Amenity(AmenityPoiCategory::DrinkingWater),
        "toilets" => PoiCategory::Amenity(AmenityPoiCategory::Toilets),
        "shelter" => PoiCategory::Amenity(AmenityPoiCategory::Shelter),
        "telephone" => PoiCategory::Amenity(AmenityPoiCategory::Telephone),
        "bank" | "atm" => PoiCategory::Shop(ShopPoiCategory::Bank),
        "pharmacy" => PoiCategory::Shop(ShopPoiCategory::Health),
        "hospital" => PoiCategory::Emergency(EmergencyPoiCategory::Hospital),
        "clinic" => PoiCategory::Shop(ShopPoiCategory::Clinic),
        "dentist" => PoiCategory::Shop(ShopPoiCategory::Clinic), // TODO: subfacet here?
        "veterinary" => PoiCategory::Shop(ShopPoiCategory::Veterinary),
        "library" => PoiCategory::Amenity(AmenityPoiCategory::Library),
        _ => PoiCategory::Address,
    });

    let house_number = tags
        .get("addr:housenumber")
        .map(|s| permute_housenum(&s).unwrap())
        .unwrap_or_default();
    let road = tags
        .get("addr:street")
        .map(|s| permute_road(&s).unwrap())
        .unwrap_or_default();
    let unit = tags
        .get("addr:unit")
        .map(|s| permute_unit(&s).unwrap())
        .unwrap_or_default();

    let names = {
        let mut names = Vec::new();
        tags.iter()
            .filter(|(key, _value)| key.contains("name:") || key.to_string() == "name")
            .for_each(|(_key, value)| {
                names.push(value.to_string());
            });
        names
    };

    if (house_number.is_empty() || road.is_empty()) && names.is_empty() {
        return None;
    }

    Some(
        AirmailPoi::new(
            names,
            "osm".to_string(),
            category.into_iter().collect(),
            house_number,
            road,
            unit,
            lat,
            lng,
            tags.iter()
                .filter_map(|(k, v)| {
                    if k.starts_with("addr:") || v.len() > 1024 {
                        None
                    } else {
                        Some((k.to_string(), v.to_string()))
                    }
                })
                .collect(),
        )
        .unwrap(),
    )
}

fn way_centroid(way: &Way) -> Option<(f64, f64)> {
    let node_positions: Vec<Coord> = way
        .nodes()
        .iter()
        .filter_map(|node| Some(Coord::from((node.lng(), node.lat()))))
        .collect();

    if node_positions.is_empty() {
        debug!("Empty node_positions: {:?}", way.id());
    }
    let linestring = LineString::new(node_positions);
    let polygon = Polygon::new(linestring, vec![]);
    let centroid = polygon.centroid();
    if centroid.is_none() {
        debug!("No centroid for way: {:?}", way.id());
    }
    let centroid = centroid?;
    Some((centroid.x(), centroid.y()))
}

fn index_way(tags: &HashMap<String, String>, way: &Way) -> Option<AirmailPoi> {
    let (lng, lat) = way_centroid(way)?;
    tags_to_poi(&tags, lat, lng)
}

fn relation_centroid(relation: &Relation, level: u32) -> Option<(f64, f64)> {
    let mut points = Vec::new();
    if level > 3 {
        debug!("Skipping relation with level > 10: {:?}", relation.id());
        return None;
    }
    for member in relation.members() {
        match member {
            RelationMember::Node(_, node) => {
                points.push(Point::new(node.lng(), node.lat()));
            }
            RelationMember::Way(_, way) => {
                if let Some(centroid) = way_centroid(&way) {
                    points.push(Point::new(centroid.0, centroid.1));
                }
            }
            RelationMember::Relation(_, relation) => {
                if let Some(centroid) = relation_centroid(&relation, level + 1) {
                    points.push(Point::new(centroid.0, centroid.1));
                } else {
                    debug!("Skipping relation with no centroid: {:?}", relation.id());
                }
            }
        }
    }
    let multipoint = MultiPoint::from(points);
    let centroid = multipoint.centroid()?;
    Some((centroid.x(), centroid.y()))
}

pub fn parse_osm<CB: Sync + Fn(AirmailPoi) -> Result<(), Box<dyn std::error::Error>>>(
    db_path: &str,
    callback: &CB,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut osm = Turbosm::open(db_path).unwrap();
    println!("Processing nodes");
    osm.process_all_nodes(|node| {
        if let Some(poi) = tags_to_poi(node.tags(), node.lat(), node.lng()) {
            if let Err(err) = callback(poi) {
                warn!("Error from callback: {}", err);
            }
        }
    })?;
    println!("Processing ways");
    osm.process_all_ways(|way| {
        index_way(way.tags(), &way).map(|poi| {
            if let Err(err) = callback(poi) {
                warn!("Error from callback: {}", err);
            }
        });
    })?;
    println!("Processing relations");
    osm.process_all_relations(|relation| {
        let centroid = relation_centroid(&relation, 0);
        if let Some(centroid) = centroid {
            if let Some(poi) = tags_to_poi(relation.tags(), centroid.1, centroid.0) {
                if let Err(err) = callback(poi) {
                    warn!("Error from callback: {}", err);
                }
            }
        }
    })?;
    println!("Done");
    Ok(())
}
