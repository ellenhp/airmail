use std::{collections::HashSet, error::Error, num::NonZeroUsize, sync::OnceLock};

use lru::LruCache;
use serde::Deserialize;
use tokio::{sync::Mutex, task::JoinHandle};

static LRU_NAMES: OnceLock<Mutex<LruCache<u64, Vec<String>>>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PipResponse {
    pub admins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConcisePipResponse {
    pub source: String,
    pub id: String,
    pub class: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PipPlaceName {
    pub lang: String,
    pub tag: String,
    pub abbr: bool,
    pub name: String,
}

thread_local! {
    static HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

// OnceLock for LRU cache
static LRU_ADMIN_AREAS: OnceLock<Mutex<LruCache<u64, Vec<u64>>>> = OnceLock::new();

async fn query_pip_inner(s2cell: u64, port: usize) -> Result<Vec<u64>, Box<dyn Error>> {
    let desired_level = 15;
    let cell = s2::cellid::CellID(s2cell);
    let cell = if cell.level() > desired_level {
        cell.parent(desired_level)
    } else {
        cell
    };

    {
        let lru_admin_areas = LRU_ADMIN_AREAS
            .get_or_init(|| Mutex::new(LruCache::new(NonZeroUsize::new(8 * 1024 * 1024).unwrap())));
        let mut lru_admin_areas = lru_admin_areas.lock().await;
        if let Some(admin_areas) = lru_admin_areas.get(&cell.0) {
            return Ok(admin_areas.clone());
        }
    }

    let lat_lng = s2::latlng::LatLng::from(cell);
    let lat = lat_lng.lat.deg();
    let lng = lat_lng.lng.deg();
    let url = format!(
        "http://localhost:{}/query/pip?lon={}&lat={}",
        port, lng, lat
    );
    let response = HTTP_CLIENT
        .with(|client: &reqwest::Client| client.get(&url).send())
        .await?;
    if response.status() != 200 {
        return Err(format!("HTTP error: {}", response.status()).into());
    }
    let response_json = response.text().await?;
    let response: Vec<ConcisePipResponse> = serde_json::from_str(&response_json)?;
    let mut response_ids = Vec::new();
    for concise_response in response {
        let admin_id: u64 = concise_response.id.parse()?;
        // Mostly here to avoid putting timezones in the index.
        if concise_response.class == "admin" {
            response_ids.push(admin_id);
        }
    }
    {
        let lru_admin_areas = LRU_ADMIN_AREAS
            .get_or_init(|| Mutex::new(LruCache::new(NonZeroUsize::new(8 * 1024 * 1024).unwrap())));
        let mut lru_admin_areas = lru_admin_areas.lock().await;
        lru_admin_areas.put(cell.0, response_ids.clone());
    }

    Ok(response_ids)
}

pub async fn query_pip(s2cell: u64, port: usize) -> Result<PipResponse, Box<dyn Error>> {
    let wof_ids = query_pip_inner(s2cell, port).await?;
    let mut handles: Vec<JoinHandle<Option<Vec<String>>>> = Vec::new();
    for admin_id in wof_ids {
        let url = format!("http://localhost:{}/place/wof/{}/name", port, &admin_id);
        let handle: JoinHandle<Option<Vec<String>>> = tokio::spawn(async move {
            {
                let lru_names = LRU_NAMES.get_or_init(|| {
                    Mutex::new(LruCache::new(NonZeroUsize::new(16 * 1024 * 1024).unwrap()))
                });
                let mut lru_names = lru_names.lock().await;
                if let Some(names) = lru_names.get(&admin_id) {
                    return Some(names.clone());
                }
            }

            let response = if let Ok(response) = HTTP_CLIENT
                .with(|client: &reqwest::Client| client.get(&url).send())
                .await
            {
                response
            } else {
                return None;
            };
            if response.status() != 200 {
                return None;
            }
            let response = if let Ok(response) = response.text().await {
                response
            } else {
                return None;
            };
            let response: Vec<PipPlaceName> = if let Ok(response) = serde_json::from_str(&response)
            {
                response
            } else {
                return None;
            };

            let names = response
                .iter()
                .filter(|place_name| place_name.tag == "preferred" || place_name.tag == "default")
                .filter(|place_name| match place_name.lang.as_str() {
                    "ara" => true, // Arabic.
                    "dan" => true, // Danish.
                    "deu" => true, // German.
                    "fra" => true, // French.
                    "fin" => true, // Finnish.
                    "hun" => true, // Hungarian.
                    "gre" => true, // Greek.
                    "ita" => true, // Italian.
                    "nld" => true, // Dutch.
                    "por" => true, // Portuguese.
                    "rus" => true, // Russian.
                    "ron" => true, // Romanian.
                    "spa" => true, // Spanish.
                    "eng" => true, // English.
                    "swe" => true, // Swedish.
                    "tam" => true, // Tamil.
                    "tur" => true, // Turkish.
                    "zho" => true, // Chinese.
                    _ => false,
                })
                .map(|place_name| deunicode::deunicode(&place_name.name).to_lowercase())
                .collect::<HashSet<_>>()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            {
                let lru_names = LRU_NAMES.get_or_init(|| {
                    Mutex::new(LruCache::new(NonZeroUsize::new(16 * 1024 * 1024).unwrap()))
                });
                let mut lru_names = lru_names.lock().await;
                lru_names.put(admin_id, names.clone());
            }
            Some(names)
        });
        handles.push(handle);
    }

    let mut response = PipResponse::default();
    for handle in handles {
        if let Ok(Some(names)) = handle.await {
            response.admins.extend(names);
        }
    }

    Ok(response)
}
