use std::{collections::HashSet, error::Error, num::NonZeroUsize, sync::OnceLock};

use lru::LruCache;
use serde::Deserialize;
use tokio::{sync::Mutex, task::JoinHandle};

static LRU_NAMES: OnceLock<Mutex<LruCache<u64, Vec<String>>>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PipResponse {
    pub locality: Option<Vec<String>>,
    pub neighbourhood: Option<Vec<String>>,
    pub county: Option<Vec<String>>,
    pub region: Option<Vec<String>>,
    pub country: Option<Vec<String>>,
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

pub async fn query_pip(lat: f64, lng: f64, port: usize) -> Result<PipResponse, Box<dyn Error>> {
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
    let mut response_names = Vec::new();
    let mut handles: Vec<JoinHandle<Option<(String, u64, Vec<String>)>>> = Vec::new();
    for concise_response in response {
        let admin_id: u64 = concise_response.id.clone().parse()?;
        {
            let lru_names = LRU_NAMES
                .get_or_init(|| Mutex::new(LruCache::new(NonZeroUsize::new(1024 * 1024).unwrap())));
            let mut lru_names = lru_names.lock().await;
            if let Some(name) = lru_names.get(&admin_id) {
                response_names.push((concise_response.r#type, admin_id, name.clone()));
                continue;
            }
        }
        let url = format!("http://localhost:{}/place/wof/{}/name", port, &admin_id);
        let handle: JoinHandle<Option<(String, u64, Vec<String>)>> = tokio::spawn(async move {
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
                .map(|place_name| place_name.name.clone())
                .collect::<HashSet<_>>()
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            {
                let lru_names = LRU_NAMES.get_or_init(|| {
                    Mutex::new(LruCache::new(NonZeroUsize::new(1024 * 1024).unwrap()))
                });
                let mut lru_names = lru_names.lock().await;
                lru_names.put(concise_response.id.parse().unwrap(), names.clone());
            }
            Some((concise_response.r#type, admin_id, names))
        });
        handles.push(handle);
    }
    for handle in handles {
        if let Ok(Some((r#type, admin_id, names))) = handle.await {
            response_names.push((r#type, admin_id, names));
        }
    }

    let mut response = PipResponse::default();
    for (r#type, _wof_id, wof_names) in response_names {
        let val = Some(wof_names);
        match r#type.as_str() {
            "locality" => {
                response.locality = val;
            }
            "neighbourhood" => {
                response.neighbourhood = val;
            }
            "county" => {
                response.county = val;
            }
            "region" => {
                response.region = val;
            }
            "country" => {
                response.country = val;
            }
            _ => {}
        }
    }

    Ok(response)
}
