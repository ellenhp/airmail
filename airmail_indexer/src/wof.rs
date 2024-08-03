use anyhow::Result;
use log::trace;
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Pool, Sqlite,
};
use std::path::Path;

/// Performs simple point-in-polygon queries against the WhosOnFirst database.
/// Queries against the WOF database require the SQLite mod_spatialite extension to be loaded.
/// Requires package: libsqlite3-mod-spatialite on Debian/Ubuntu
#[derive(Clone)]
pub struct WhosOnFirst {
    pool: Pool<Sqlite>,
}

impl WhosOnFirst {
    /// Opens a connection to the WhosOnFirst database.
    /// Requires the SQLite mod_spatialite extension to be loaded.
    pub async fn new(path: &Path) -> Result<Self> {
        trace!("Opening WhosOnFirst database at {:?}", path);

        let opts = SqliteConnectOptions::new()
            .filename(path)
            .journal_mode(SqliteJournalMode::Wal)
            .pragma("cache_size", "2000")
            .pragma("synchronous", "OFF")
            .pragma("temp_store", "MEMORY")
            .pragma("foreign_keys", "OFF")
            .pragma("recursive_triggers", "OFF")
            .pragma("locking_mode", "NORMAL")
            .extension("mod_spatialite");

        // Connections with the total number of physical and virtual cores.
        // The sqlx pool isn't the most efficient, so keep it busy.
        let connections = num_cpus::get().try_into()?;

        let pool = SqlitePoolOptions::new()
            .min_connections(connections)
            .max_connections(connections)
            .connect_with(opts)
            .await?;

        Ok(Self { pool })
    }

    /// Returns the WOF ID of polygons that contain the given point.
    /// Requires the spatialite extension to be loaded.
    pub async fn point_in_polygon(&self, lon: f64, lat: f64) -> Result<Vec<ConcisePipResponse>> {
        let lon: f32 = lon as f32;
        let lat: f32 = lat as f32;
        let rows = sqlx::query_as::<_, ConcisePipResponse>(
            r"
                SELECT place.source, place.id, place.class, place.type
                FROM main.point_in_polygon
                LEFT JOIN place USING (source, id)
                WHERE search_frame = MakePoint( ?1, ?2, 4326 )
                AND INTERSECTS( point_in_polygon.geom, MakePoint( ?1, ?2, 4326 ) )
                AND place.source IS NOT NULL
                AND (
                    place.type != 'planet'
                    AND place.type != 'marketarea'
                    AND place.type != 'county'
                    AND place.type != 'timezone'
                )
                LIMIT 1000
            ",
        )
        .bind(lon)
        .bind(lat)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Lookup the name of a place by its WOF ID.
    pub async fn place_name_by_id(&self, id: u64) -> Result<Vec<PipPlaceName>> {
        // Convert to i64 for SQLite
        let id: i64 = id.try_into()?;

        // Index for name is on (source, id)
        let rows = sqlx::query_as::<_, PipPlaceName>(
            r"
                SELECT name.lang, name.tag, name.abbr, name.name
                FROM main.name
                WHERE name.source = 'wof'
                AND name.id = ?1
                AND name.tag IN ('preferred', 'default')
                AND name.lang IN (
                    'ara', 'dan', 'deu', 'fra', 'fin', 'hun', 'gre', 'ita', 'nld', 'por',
                    'rus', 'ron', 'spa', 'eng', 'swe', 'tam', 'tur', 'zho'
                )
            ",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Lookup the properties of a place by its WOF ID.
    pub async fn properties_for_id(&self, id: u64) -> Result<Vec<WofKV>> {
        // Convert to i64 for SQLite
        let id: i64 = id.try_into()?;

        // Index for name is on (source, id)
        let rows = sqlx::query_as::<_, WofKV>(
            r"
                SELECT property.key, property.value
                FROM main.property
                WHERE property.source = 'wof'
                AND property.id = ?1
                AND property.key = 'wof:lang_x_spoken'
            ",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Retrieve a flat representation of all polygons in the database.
    /// This call can be 10GB+ of data.
    pub async fn all_polygons(&self) -> Result<Vec<PipWithGeometry>> {
        // Geometry is stored as spatialite blob, so decode to WKB (geopackage compatible).
        let rows = sqlx::query_as::<_, PipWithGeometry>(
            r"
                SELECT
                    place.source,
                    place.id,
                    place.class,
                    place.type,
                    AsGPB(shard.geom) as geom
                FROM shard
                LEFT JOIN place USING (source, id)
                WHERE place.source IS NOT NULL
                AND (
                    place.type != 'planet'
                    AND place.type != 'marketarea'
                    AND place.type != 'county'
                    AND place.type != 'timezone'
                )
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

/// A key-value pair from the WhosOnFirst database.
#[derive(Debug, Clone, Deserialize, sqlx::FromRow)]
pub struct WofKV {
    key: String,
    value: String,
}

/// A concise representation of a place in the WhosOnFirst database.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ConcisePipResponse {
    /// WOF data source, usually wof
    pub source: String,

    /// WOF ID of the place
    pub id: String,

    /// High level bucket of human activity - https://whosonfirst.org/docs/categories/
    /// POINT-OF-VIEW > CLASS > CATEGORY
    pub class: String,

    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize, sqlx::FromRow)]
pub struct PipPlaceName {
    pub lang: String,
    pub tag: String,
    // #[allow(dead_code)]
    // pub abbr: bool,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PipLangsResponse {
    #[serde(rename = "wof:lang_x_spoken")]
    pub langs: Option<String>,
}

/// Represents a place in the WhosOnFirst database with a geometry.
#[derive(sqlx::FromRow)]
pub struct PipWithGeometry {
    /// WOF data source, usually wof
    pub source: String,

    /// WOF ID of the place
    pub id: String,

    /// High level bucket of human activity - https://whosonfirst.org/docs/categories/
    /// POINT-OF-VIEW > CLASS > CATEGORY
    pub class: String,

    pub r#type: String,

    pub geom: geozero::wkb::Decode<geo_types::Geometry<f64>>,
}

/// Convert from a list of key-value pairs to a PipLangsResponse.
impl From<Vec<WofKV>> for PipLangsResponse {
    fn from(value: Vec<WofKV>) -> Self {
        let mut langs = None;
        for kv in value {
            if kv.key == "wof:lang_x_spoken" {
                langs = Some(kv.value);
            }
        }
        Self { langs }
    }
}

/// Deconstruct a PipWithGeometry into a geometry and a concise response.
impl From<PipWithGeometry> for (Option<geo_types::Geometry<f64>>, ConcisePipResponse) {
    fn from(value: PipWithGeometry) -> Self {
        (
            value.geom.geometry,
            ConcisePipResponse {
                source: value.source,
                id: value.id,
                class: value.class,
                r#type: value.r#type,
            },
        )
    }
}
