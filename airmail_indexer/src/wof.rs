use anyhow::Result;
use log::debug;
use serde::Deserialize;
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
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .journal_mode(SqliteJournalMode::Wal)
            .pragma("cache_size", "1000000")
            .pragma("synchronous", "OFF")
            .pragma("temp_store", "MEMORY")
            .pragma("mmap_size", "1073741824") // 1GB, virtual memory
            .pragma("foreign_keys", "OFF")
            .pragma("recursive_triggers", "OFF")
            .pragma("page_size", "32768")
            .pragma("locking_mode", "NORMAL")
            .extension("mod_spatialite");

        let connections = num_cpus::get_physical().try_into()?;

        let pool = SqlitePoolOptions::new()
            .min_connections(connections)
            .max_connections(connections)
            .connect_with(opts)
            .await?;

        debug!("Connected to WhosOnFirst database at {:?}", path);

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
}

#[derive(Debug, Clone, Deserialize, sqlx::FromRow)]
pub struct WofKV {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize, sqlx::FromRow)]
pub struct ConcisePipResponse {
    // #[allow(dead_code)]
    // pub source: String,
    pub id: String,
    // #[allow(dead_code)]
    // pub class: String,
    #[serde(rename = "type")]
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
