use anyhow::Result;
use env_logger::Env;
use std::path::Path;

use log::debug;

use crate::wof::WhosOnFirst;

#[tokio::test]
async fn wof_read() -> Result<()> {
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
        .is_test(true)
        .try_init();

    // Connect to the WhosOnFirst database.
    // Ensuring the database is present, and the mod_spatialite extension is loaded.
    let wof = WhosOnFirst::new(Path::new(
        "../data/whosonfirst-data-admin-latest.spatial.db",
    ))
    .await?;

    // Test point_in_polygon.
    // This should exist in Global and Australia.
    let pip = wof
        .point_in_polygon(150.93800658307805, -33.88246407738443)
        .await?;
    debug!("point_in_polygon: {:?}", pip);
    assert!(!pip.is_empty());

    // Lookup a place by ID, this should return Australia/Sydney
    let place_name = wof.place_name_by_id(102047721).await?;
    debug!("place_name: {:?}", place_name);
    assert!(!place_name.is_empty());

    // Lookup a country
    let country = wof.properties_for_id(85632793).await?;
    debug!("country: {:?}", country);

    Ok(())
}
