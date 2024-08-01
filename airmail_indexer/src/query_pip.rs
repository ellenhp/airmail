use std::collections::HashSet;

use anyhow::Result;
use crossbeam::channel::Sender;
use futures_util::future::join_all;
use serde::Deserialize;

use crate::{
    cache::{IndexerCache, WofCacheItem},
    pip_tree::PipTree,
    wof::{ConcisePipResponse, PipLangsResponse, WhosOnFirst},
    COUNTRIES,
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

async fn query_pip_inner(
    s2cell: u64,
    indexer_cache: &IndexerCache,
    to_cache_sender: Sender<WofCacheItem>,
    wof_db: &WhosOnFirst,
    pip_tree: &Option<PipTree<ConcisePipResponse>>,
) -> Result<AdminIds> {
    let desired_level = 15;
    let cell = s2::cellid::CellID(s2cell);
    let cell = if cell.level() > desired_level {
        cell.parent(desired_level)
    } else {
        cell
    };

    let ids = indexer_cache.query_area(cell.0)?;
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

    // Prefer the pip_tree, if in use
    let response = if let Some(pip_tree) = pip_tree {
        pip_tree.point_in_polygon(lng, lat).await?
    } else {
        wof_db.point_in_polygon(lng, lat).await?
    };

    let mut response_ids = Vec::new();
    for concise_response in response {
        let admin_id: u64 = concise_response.id.parse()?;

        // These filters are applied in SQL, to reduce allocations
        // if concise_response.r#type == "planet"
        //     || concise_response.r#type == "marketarea"
        //     || concise_response.r#type == "county"
        //     || concise_response.r#type == "timezone"
        // {
        //     continue;
        // }
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

async fn query_names(admin_id: u64, wof_db: &WhosOnFirst) -> Option<(u64, Vec<String>)> {
    let response = wof_db.place_name_by_id(admin_id).await.ok()?;
    if response.is_empty() {
        return None;
    }
    let names = response
        .iter()
        // These languages and filters are also applied in SQL
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

    Some((admin_id, names))
}

async fn query_langs(country_id: u64, wof_db: &WhosOnFirst) -> Option<(u64, Vec<String>)> {
    let response: PipLangsResponse = wof_db.properties_for_id(country_id).await.ok()?.into();
    let langs: Vec<String> = response
        .langs
        .map(|langs| langs.split(',').map(|s| s.to_string()).collect())?;

    Some((country_id, langs))
}

pub(crate) async fn query_pip(
    indexer_cache: &IndexerCache,
    to_cache_sender: Sender<WofCacheItem>,
    s2cell: u64,
    wof_db: &WhosOnFirst,
    pip_tree: &Option<PipTree<ConcisePipResponse>>,
) -> Result<PipResponse> {
    let wof_ids = query_pip_inner(
        s2cell,
        indexer_cache,
        to_cache_sender.clone(),
        wof_db,
        pip_tree,
    )
    .await?;
    let mut response = PipResponse::default();
    let mut admin_name_futures = vec![];
    let mut lang_futures = vec![];

    // Query names for the admin areas
    for admin_id in wof_ids.all_admin_ids {
        // This check was at the end, but I think it should be here as the ID has already been looked up
        if COUNTRIES.contains(&admin_id) {
            continue;
        }

        if let Ok(Some(names)) = indexer_cache.query_names_cache(admin_id) {
            response.admin_names.extend(names);
        } else {
            admin_name_futures.push(query_names(admin_id, wof_db));
        }
    }

    // Query languages for the country
    if let Some(country_id) = wof_ids.country {
        if let Ok(Some(langs)) = indexer_cache.query_languages_cache(country_id) {
            response.admin_langs.extend(langs);
        } else {
            lang_futures.push(query_langs(country_id, wof_db));
        }
    }

    // Drive the futures to completion
    for (admin_id, names) in join_all(admin_name_futures).await.into_iter().flatten() {
        to_cache_sender
            .send(WofCacheItem::Names(admin_id, names.clone()))
            .unwrap();
        response.admin_names.extend(names);
    }

    for (country_id, langs) in join_all(lang_futures).await.into_iter().flatten() {
        to_cache_sender
            .send(WofCacheItem::Langs(country_id, langs.clone()))
            .unwrap();
        response.admin_langs.extend(langs);
    }

    Ok(response)
}
