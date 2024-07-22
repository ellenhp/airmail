use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OpenFlags;
use serde::Deserialize;
use std::path::Path;

/// Performs simple point-in-polygon queries against the WhosOnFirst database.
///
/// # Safety
/// Querying the WhosOnFirst database requires the spatialite extension to be loaded
/// which requires use of unsafe to load the dylib. After the extension is loaded/fails
/// it is disabled to prevent other connections from loading extensions.
///
#[derive(Clone)]
pub struct WhosOnFirst {
    pool: Pool<SqliteConnectionManager>,
}

impl WhosOnFirst {
    /// Opens a connection to the WhosOnFirst database.
    /// Requires package: libsqlite3-mod-spatialite on Debian/Ubuntu
    pub fn new(path: &Path) -> Result<Self> {
        let conn_man = SqliteConnectionManager::file(path)
            .with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)
            .with_init(|c| {

                // Enable spatialite extension.
                // Unsafe is used to permit the use of the spatialite extension, per documentation
                // after the extension is loaded, it is disabled to prevent other connections from using it.
                unsafe {
                    c.load_extension_enable()?;
                    let load_attempt = c.load_extension("mod_spatialite", None);
                    if let Err(e) = &load_attempt {
                        eprintln!("Failed to load mod_spatialite: {:?}. libsqlite3-mod-spatialite is needed on Debian systems.", e);
                    }
                    c.load_extension_disable()?;
                    load_attempt
                }?;

                c.execute_batch(
                    r"
                        PRAGMA cache_size = 2000;
                        PRAGMA temp_store = MEMORY;
                        PRAGMA mmap_size = 268435456;
                        PRAGMA foreign_keys = OFF;
                    ",
                )?;

                Ok(())
            });

        let pool = Pool::builder()
            .max_size(num_cpus::get().try_into()?)
            .build(conn_man)?;

        Ok(Self { pool })
    }

    pub fn point_in_polygon(&self, lon: f64, lat: f64) -> Result<Vec<ConcisePipResponse>> {
        let connection = self.pool.get()?;

        // Requires the spatialite extension to be loaded.
        let mut statement = connection.prepare_cached(
            r"
                SELECT place.source, place.id, place.class, place.type
                FROM main.point_in_polygon AS pip
                LEFT JOIN place USING (source, id)
                WHERE search_frame = MakePoint( ?1, ?2, 4326 )
                AND INTERSECTS( pip.geom, MakePoint( ?1, ?2, 4326 ) )
                AND place.source IS NOT NULL
            ",
        )?;

        let rows = statement
            .query_map([lon, lat], |row| {
                Ok(ConcisePipResponse {
                    source: row.get(0)?,
                    id: row.get(1)?,
                    class: row.get(2)?,
                    r#type: row.get(3)?,
                })
            })?
            .flatten()
            .collect();

        Ok(rows)
    }

    pub fn place_name_by_id(&self, id: u64) -> Result<Vec<PipPlaceName>> {
        let connection = self.pool.get()?;

        // Index for name is on (source, id)
        let mut statement = connection.prepare_cached(
            r"
                SELECT name.lang, name.tag, name.abbr, name.name
                FROM main.name
                WHERE name.source = 'wof'
                AND name.id = ?1
            ",
        )?;

        let rows = statement
            .query_map([id], |row| {
                Ok(PipPlaceName {
                    lang: row.get(0)?,
                    tag: row.get(1)?,
                    abbr: row.get(2)?,
                    name: row.get(3)?,
                })
            })?
            .flatten()
            .collect();

        Ok(rows)
    }

    pub fn properties_for_id(&self, id: u64) -> Result<Vec<WofKV>> {
        let connection = self.pool.get()?;

        // Index for name is on (source, id)
        let mut statement = connection.prepare_cached(
            r"
                SELECT property.key, property.value
                FROM main.property
                WHERE property.source = 'wof'
                AND property.id = ?1
            ",
        )?;

        let rows = statement
            .query_map([id], |row| {
                Ok(WofKV {
                    key: row.get(0)?,
                    value: row.get(1)?,
                })
            })?
            .flatten()
            .collect();

        Ok(rows)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WofKV {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConcisePipResponse {
    #[allow(dead_code)]
    pub source: String,
    pub id: String,
    #[allow(dead_code)]
    pub class: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PipPlaceName {
    pub lang: String,
    pub tag: String,
    #[allow(dead_code)]
    pub abbr: bool,
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
