use std::{collections::HashMap, fs::File};

use airmail::poi::AirmailPoi;
use airmail_common::categories::{
    AmenityPoiCategory, EmergencyPoiCategory, FoodPoiCategory, PoiCategory, ShopPoiCategory,
};
use geo::{Centroid, Coord, LineString, Polygon};
use log::warn;
use osmpbf::Element;

use crate::substitutions::{permute_housenum, permute_road, permute_unit};

fn tags_to_poi(tags: &HashMap<&str, &str>, lat: f64, lng: f64) -> Option<AirmailPoi> {
    if tags.contains_key("highway") {
        return None;
    }

    let category = tags.get("amenity").map(|s| match *s {
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
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
        .unwrap(),
    )
}

pub fn parse_osm<CB: Sync + Fn(AirmailPoi) -> Result<(), Box<dyn std::error::Error>>>(
    pbf_path: &str,
    callback: &CB,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(pbf_path)?;
    let reader = osmpbf::reader::ElementReader::new(file);
    let ways_of_interest = reader.par_map_reduce(
        |obj| match obj {
            osmpbf::Element::Node(node) => {
                let tags = node.tags().clone().collect();
                let lat = node.lat();
                let lng = node.lon();
                if let Some(poi) = tags_to_poi(&tags, lat, lng) {
                    if let Err(err) = callback(poi) {
                        warn!("Error: {}", err);
                    }
                }
                vec![]
            }
            osmpbf::Element::DenseNode(densenode) => {
                let tags = densenode.tags().clone().collect();
                let lat = densenode.lat();
                let lng = densenode.lon();
                if let Some(poi) = tags_to_poi(&tags, lat, lng) {
                    if let Err(err) = callback(poi) {
                        warn!("Error: {}", err);
                    }
                }
                vec![]
            }
            osmpbf::Element::Way(way) => {
                let tags = way.tags().clone().collect();
                let node_positions: Vec<Coord> = way
                    .node_locations()
                    .map(|node| Coord {
                        x: node.lon(),
                        y: node.lat(),
                    })
                    .collect();
                if node_positions.len() < 3 {
                    return vec![];
                }
                let linestring = LineString::new(node_positions);
                let polygon = Polygon::new(linestring, vec![]);
                let (lng, lat) = polygon.centroid().unwrap().into();
                if let Some(poi) = tags_to_poi(&tags, lat, lng) {
                    if let Err(err) = callback(poi) {
                        warn!("Error: {}", err);
                    }
                }
                vec![]
            }
            osmpbf::Element::Relation(relation) => {
                if let Some(outer) = relation.members().next() {
                    let tags: HashMap<String, String> = relation
                        .tags()
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .collect();
                    return vec![(outer.member_id, tags)];
                }
                vec![]
            }
        },
        || vec![],
        |a, b| {
            let mut a = a.clone();
            a.extend(b);
            a
        },
    )?;

    let file = File::open(pbf_path)?;
    let reader = osmpbf::reader::ElementReader::new(file);
    let ways_of_interest = ways_of_interest.into_iter().collect::<HashMap<_, _>>();
    reader.for_each(|obj| {
        if let Element::Way(way) = obj {
            if ways_of_interest.contains_key(&way.id()) {
                let tags = ways_of_interest.get(&way.id()).unwrap();
                let node_positions: Vec<Coord> = way
                    .node_locations()
                    .map(|node| Coord {
                        x: node.lon(),
                        y: node.lat(),
                    })
                    .collect();
                if node_positions.len() < 3 {
                    return;
                }
                let linestring = LineString::new(node_positions);
                let polygon = Polygon::new(linestring, vec![]);
                let (lng, lat) = polygon.centroid().unwrap().into();
                let tags_str = tags.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                if let Some(poi) = tags_to_poi(&tags_str, lat, lng) {
                    if let Err(err) = callback(poi) {
                        warn!("Error: {}", err);
                    }
                }
            }
        }
    })?;
    Ok(())
}
