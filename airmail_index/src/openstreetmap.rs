use std::{collections::HashMap, error::Error, ops::Range};

use airmail::poi::AirmailPoi;
use airmail_common::categories::{
    AmenityPoiCategory, CuisineCategory, EmergencyPoiCategory, FoodPoiCategory, PoiCategory,
    ShopPoiCategory,
};
use geo::{Centroid, Coord, LineString, Polygon};
use log::{debug, warn};
use osmflat::{FileResourceStorage, Osm, Way, COORD_SCALE};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::substitutions::permute_road;

fn tags_to_poi(tags: &HashMap<String, String>, lat: f64, lng: f64) -> Option<AirmailPoi> {
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

    let category = tags
        .get("amenity")
        .map(|s| match s.as_str() {
            "fast_food" | "food_court" | "cafe" | "pub" | "restaurant" => {
                if let Some(cuisine) = tags.get("cuisine") {
                    let cuisine = match cuisine.as_str() {
                        "burger" | "hot_dog" | "american" => CuisineCategory::American,
                        "coffee_shop" => CuisineCategory::CoffeeShop,
                        "pizza" => CuisineCategory::Pizza,
                        "chinese" | "indian" | "vietnamese" | "japanese" | "thai" => {
                            CuisineCategory::Asian
                        }
                        _ => CuisineCategory::Other {
                            raw_tag: cuisine.clone(),
                        },
                    };
                    PoiCategory::Shop(ShopPoiCategory::Food(FoodPoiCategory::Restaurant(Some(
                        cuisine,
                    ))))
                } else {
                    PoiCategory::Shop(ShopPoiCategory::Food(FoodPoiCategory::Restaurant(None)))
                }
            }
            "biergarten" | "bar" => PoiCategory::Shop(ShopPoiCategory::Bar),
            "drinking_water" => PoiCategory::Amenity(AmenityPoiCategory::DrinkingWater),
            "toilets" => PoiCategory::Amenity(AmenityPoiCategory::Toilets),
            "shelter" => PoiCategory::Amenity(AmenityPoiCategory::Shelter),
            "telephone" => PoiCategory::Amenity(AmenityPoiCategory::Telephone),
            "bank" | "atm" => PoiCategory::Shop(ShopPoiCategory::Bank),
            "pharmacy" => PoiCategory::Shop(ShopPoiCategory::Health),
            "hospital" => PoiCategory::Emergency(EmergencyPoiCategory::Hospital),
            "clinic" => PoiCategory::Shop(ShopPoiCategory::Clinic),
            "dentist" => PoiCategory::Shop(ShopPoiCategory::Dentist), // TODO: subfacet here?
            "veterinary" => PoiCategory::Shop(ShopPoiCategory::Veterinary),
            "library" => PoiCategory::Amenity(AmenityPoiCategory::Library),
            _ => PoiCategory::Address,
        })
        .unwrap_or(PoiCategory::Address);

    let house_number = tags
        .get("addr:housenumber")
        .map(|s| vec![s.to_string()])
        .unwrap_or_default();
    let road = tags
        .get("addr:street")
        .map(|s| permute_road(&s).unwrap())
        .unwrap_or_default();
    let unit = tags
        .get("addr:unit")
        .map(|s| vec![s.to_string()])
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
            category,
            house_number,
            road,
            unit,
            lat,
            lng,
            tags.into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .unwrap(),
    )
}

fn way_centroid(way: &Way, osm: &Osm) -> Option<(f64, f64)> {
    let node_positions: Vec<Coord> = way
        .refs()
        .filter_map(|node| {
            let node = &osm.nodes_index()[node as usize];
            let node = &osm.nodes()[node.value().unwrap() as usize];
            Some(Coord::from((
                node.lon() as f64 / COORD_SCALE as f64,
                node.lat() as f64 / COORD_SCALE as f64,
            )))
        })
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

fn index_way(tags: &HashMap<String, String>, way: &Way, osm: &Osm) -> Option<AirmailPoi> {
    let (lng, lat) = way_centroid(way, osm)?;
    tags_to_poi(&tags, lat, lng)
}

fn tags(idxs: Range<u64>, osm: &Osm) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut tags = HashMap::new();
    for tag in idxs {
        let tag = &osm.tags_index()[tag as usize];
        let tag = &osm.tags()[tag.value() as usize];
        let key_idx = tag.key_idx() as usize;
        // pull out the null-terminated string.
        let key: Vec<u8> = osm.stringtable()[key_idx..]
            .iter()
            .take_while(|&&c| c != 0)
            .cloned()
            .collect();
        let key = String::from_utf8(key)?;

        let value: Vec<u8> = osm.stringtable()[tag.value_idx() as usize..]
            .iter()
            .take_while(|&&c| c != 0)
            .cloned()
            .collect();
        let value = String::from_utf8(value)?;
        tags.insert(key, value);
    }
    Ok(tags)
}

pub fn parse_osm<CB: Sync + Fn(AirmailPoi) -> Result<(), Box<dyn std::error::Error>>>(
    db_path: &str,
    callback: &CB,
) -> Result<(), Box<dyn std::error::Error>> {
    let storage = FileResourceStorage::new(db_path);
    let osm = Osm::open(storage).unwrap();
    println!("Processing nodes");
    osm.nodes().par_iter().for_each(|node| {
        let tags = tags(node.tags(), &osm);
        if let Ok(tags) = tags {
            if let Some(poi) = tags_to_poi(
                &tags,
                node.lat() as f64 / COORD_SCALE as f64,
                node.lon() as f64 / COORD_SCALE as f64,
            ) {
                if let Err(err) = callback(poi) {
                    warn!("Error from callback: {}", err);
                }
            }
        }
    });
    println!("Processing ways");
    osm.ways().par_iter().for_each(|way| {
        let tags = tags(way.tags(), &osm);
        if let Ok(tags) = tags {
            index_way(&tags, &way, &osm).map(|poi| {
                if let Err(err) = callback(poi) {
                    warn!("Error from callback: {}", err);
                }
            });
        }
    });
    println!("Skipping relations (FIXME)");
    // osm.process_all_relations(|relation, turbosm| {
    //     let centroid = relation_centroid(&relation, 0, turbosm);
    //     if let Ok(centroid) = centroid {
    //         if let Some(poi) = tags_to_poi(relation.tags(), centroid.1, centroid.0) {
    //             if let Err(err) = callback(poi) {
    //                 warn!("Error from callback: {}", err);
    //             }
    //         }
    //     }
    // })?;
    println!("Done");
    Ok(())
}
