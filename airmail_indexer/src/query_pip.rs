use std::{collections::HashSet, error::Error};

use crossbeam::channel::Sender;
use redb::{ReadTransaction, ReadableTable};
use serde::Deserialize;
use tokio::task::JoinHandle;

use crate::{WofCacheItem, ADMIN_AREAS, ADMIN_NAMES};

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

async fn query_pip_inner(
    s2cell: u64,
    read: &'_ ReadTransaction<'_>,
    to_cache_sender: Sender<WofCacheItem>,
    port: usize,
) -> Result<Vec<u64>, Box<dyn Error>> {
    let desired_level = 15;
    let cell = s2::cellid::CellID(s2cell);
    let cell = if cell.level() > desired_level {
        cell.parent(desired_level)
    } else {
        cell
    };

    let ids = {
        let mut ids: Vec<u64> = Vec::new();
        let txn = read;
        let table = txn.open_table(ADMIN_AREAS)?;
        if let Some(admin_ids) = table.get(&cell.0)? {
            for admin_id in admin_ids.value().chunks(8) {
                ids.push(u64::from_le_bytes([
                    admin_id[0],
                    admin_id[1],
                    admin_id[2],
                    admin_id[3],
                    admin_id[4],
                    admin_id[5],
                    admin_id[6],
                    admin_id[7],
                ]));
            }
        }
        ids
    };
    if !ids.is_empty() {
        return Ok(ids);
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
        if concise_response.r#type == "planet"
            || concise_response.r#type == "marketarea"
            || concise_response.r#type == "county"
            || concise_response.r#type == "timezone"
        {
            continue;
        }
        response_ids.push(admin_id);
    }

    {
        to_cache_sender
            .send(WofCacheItem::Admins(cell.0, response_ids.clone()))
            .unwrap();
    }

    Ok(response_ids)
}

fn query_names(read: &'_ ReadTransaction<'_>, admin: &u64) -> Result<Vec<String>, Box<dyn Error>> {
    let txn = read;
    let table = txn.open_table(ADMIN_NAMES)?;
    if let Some(names_ref) = table.get(admin)? {
        let names = names_ref.value().to_string();
        let names = names
            .split('\0')
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        return Ok(names);
    }
    Err("No names found".into())
}

pub(crate) async fn query_pip(
    read: &'_ ReadTransaction<'_>,
    to_cache_sender: Sender<WofCacheItem>,
    s2cell: u64,
    port: usize,
) -> Result<PipResponse, Box<dyn Error>> {
    let wof_ids = query_pip_inner(s2cell, read, to_cache_sender.clone(), port).await?;
    let mut handles: Vec<(u64, JoinHandle<Option<Vec<String>>>)> = Vec::new();
    let mut cached_names = Vec::new();
    for admin_id in wof_ids {
        let url = format!("http://localhost:{}/place/wof/{}/name", port, &admin_id);

        if let Ok(names) = query_names(read, &admin_id) {
            cached_names.extend(names);
            continue;
        }
        let handle: JoinHandle<Option<Vec<String>>> = tokio::spawn(async move {
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
            Some(names)
        });
        handles.push((admin_id, handle));
    }

    let mut response = PipResponse::default();
    response.admins.extend(cached_names);
    for (admin_id, handle) in handles {
        if let Ok(Some(names)) = handle.await {
            to_cache_sender
                .send(WofCacheItem::Names(admin_id, names.clone()))
                .unwrap();
            response.admins.extend(names);
        }
    }

    Ok(response)
}
