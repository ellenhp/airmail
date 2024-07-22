use std::path::Path;

use log::debug;

use crate::wof::WhosOnFirst;

#[test]
fn wof_read() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Connect to the WhosOnFirst database.
    // Ensuring the database is present, and the mod_spatialite extension is loaded.
    let wof = WhosOnFirst::new(Path::new(
        "../data/whosonfirst-data-admin-latest.spatial.db",
    ))
    .expect("Failed to open WhosOnFirst database.");

    // Test point_in_polygon.
    // This should exist in Global and Australia.
    let pip = wof
        .point_in_polygon(150.93800658307805, -33.88246407738443)
        .unwrap();
    debug!("point_in_polygon: {:?}", pip);
    assert!(!pip.is_empty());

    // Lookup a place by ID, this should return Australia/Sydney
    let place_name = wof
        .place_name_by_id(102047721)
        .expect("Failed to get name by ID.");
    debug!("place_name: {:?}", place_name);
    assert!(!place_name.is_empty());

    // Lookup a country
    let country = wof
        .properties_for_id(85632793)
        .expect("Failed to get country.");
    debug!("country: {:?}", country);
}
