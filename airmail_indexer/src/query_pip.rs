use std::{collections::HashSet, error::Error};

use crossbeam::channel::Sender;
use redb::{ReadTransaction, ReadableTable};
use serde::Deserialize;

use crate::{
    wof::{PipLangsResponse, WhosOnFirst},
    WofCacheItem, COUNTRIES, TABLE_AREAS, TABLE_LANGS, TABLE_NAMES,
};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PipResponse {
    pub admin_names: Vec<String>,
    pub admin_langs: Vec<String>,
}

thread_local! {
    static HTTP_CLIENT: reqwest::Client = reqwest::Client::new();
}

struct AdminIds {
    all_admin_ids: Vec<u64>,
    country: Option<u64>,
}

fn query_pip_inner(
    s2cell: u64,
    read: &'_ ReadTransaction<'_>,
    to_cache_sender: Sender<WofCacheItem>,
    wof_db: &WhosOnFirst,
) -> Result<AdminIds, Box<dyn Error>> {
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
        let table = txn.open_table(TABLE_AREAS)?;
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
        let country = ids.iter().find(|id| COUNTRIES.contains(*id)).cloned();
        return Ok(AdminIds {
            all_admin_ids: ids,
            country,
        });
    }

    let lat_lng = s2::latlng::LatLng::from(cell);
    let lat = lat_lng.lat.deg();
    let lng = lat_lng.lng.deg();
    let response = wof_db.point_in_polygon(lng, lat)?;
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

    Ok(AdminIds {
        country: response_ids
            .iter()
            .find(|id| COUNTRIES.contains(id))
            .cloned(),
        all_admin_ids: response_ids,
    })
}

fn query_names_cache(
    read: &'_ ReadTransaction<'_>,
    admin: &u64,
) -> Result<Vec<String>, Box<dyn Error>> {
    let txn = read;
    let table = txn.open_table(TABLE_NAMES)?;
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

fn query_languages_cache(
    read: &'_ ReadTransaction<'_>,
    admin: &u64,
) -> Result<Vec<String>, Box<dyn Error>> {
    let txn = read;
    let table = txn.open_table(TABLE_LANGS)?;
    if let Some(langs_ref) = table.get(admin)? {
        let langs = langs_ref.value().to_string();
        let langs = langs
            .split('\0')
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        return Ok(langs);
    }
    Err("No langs found".into())
}

fn query_names(admin_id: u64, wof_db: &WhosOnFirst) -> Option<Vec<String>> {
    let response = wof_db.place_name_by_id(admin_id).ok()?;
    if response.is_empty() {
        return None;
    }
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
}

fn query_langs(country_id: u64, wof_db: &WhosOnFirst) -> Option<Vec<String>> {
    let response: PipLangsResponse = wof_db.properties_for_id(country_id).ok()?.into();
    response
        .langs
        .map(|langs| langs.split(',').map(|s| s.to_string()).collect())
}

pub(crate) fn query_pip(
    read: &'_ ReadTransaction<'_>,
    to_cache_sender: Sender<WofCacheItem>,
    s2cell: u64,
    wof_db: &WhosOnFirst,
) -> Result<PipResponse, Box<dyn Error>> {
    let wof_ids = query_pip_inner(s2cell, read, to_cache_sender.clone(), wof_db)?;
    let mut cached_names = Vec::new();
    let mut cached_langs = Vec::new();
    let mut response = PipResponse::default();

    for admin_id in wof_ids.all_admin_ids {
        if COUNTRIES.contains(&admin_id) {
            continue;
        }

        if let Ok(names) = query_names_cache(read, &admin_id) {
            cached_names.extend(names);
        } else if let Some(names) = query_names(admin_id, wof_db) {
            to_cache_sender
                .send(WofCacheItem::Names(admin_id, names.clone()))
                .unwrap();
            response.admin_names.extend(names);
        }
    }

    if let Some(country_id) = wof_ids.country {
        if let Ok(langs) = query_languages_cache(read, &country_id) {
            cached_langs.extend(langs);
        } else if let Some(langs) = query_langs(country_id, wof_db) {
            to_cache_sender
                .send(WofCacheItem::Langs(country_id, langs.clone()))
                .unwrap();
            response.admin_langs.extend(langs);
        }
    }

    response.admin_names.extend(cached_names);
    response.admin_langs.extend(cached_langs);
    Ok(response)
}
