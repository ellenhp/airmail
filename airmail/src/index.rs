use std::{collections::HashMap, str::FromStr};

use log::error;
use roaring::RoaringBitmap;
use sqlx::{postgres::PgConnectOptions, query, Connection, PgConnection};

use crate::poi::SchemafiedPoi;

use s2::{cell::Cell, cellid::CellID, latlng::LatLng};
use std::f64::consts::PI;

fn get_quadkey(lat: f64, lng: f64, zoom: u32) -> u64 {
    let x = (lng + 180.0) / 360.0 * (1 << zoom) as f64;
    let x = x.floor() as u32;

    let lat_rad = lat.to_radians();
    let y = 1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos()).ln() / PI) / 2.0 * (1 << zoom) as f64;
    let y = y.floor() as u32;

    let mut quadkey = 0_u64;

    for i in (1..=zoom).rev() {
        let mask = 1 << (i - 1);
        let mut digit = 0;

        if x & mask != 0 {
            digit += 1;
        }
        if y & mask != 0 {
            digit += 2;
        }

        quadkey = (quadkey << 2) | digit;
    }

    quadkey
}

pub struct AirmailIndex {}

#[cfg(feature = "build_index")]
pub struct AirmailIndexBuilder {
    conn: PgConnection,
    keyword_index: HashMap<String, RoaringBitmap>,
    keyword_frequencies: HashMap<String, usize>,
    keyword_indices: HashMap<String, usize>,
    keywords: Vec<String>,
}

impl AirmailIndexBuilder {
    pub async fn create(db_url: &str) -> Result<AirmailIndexBuilder, anyhow::Error> {
        let mut conn = PgConnection::connect_with(&PgConnectOptions::from_str(db_url)?).await?;

        query!("SET synchronous_commit TO OFF")
            .execute(&mut conn)
            .await?;

        Ok(AirmailIndexBuilder {
            conn,
            keyword_index: HashMap::new(),
            keyword_frequencies: HashMap::new(),
            keyword_indices: HashMap::new(),
            keywords: Vec::new(),
        })
    }

    pub async fn collect_vocabulary(&mut self, poi: &SchemafiedPoi) -> Result<(), anyhow::Error> {
        let cell = Cell::from(CellID(poi.s2cell));
        let latlng = LatLng::from(cell.center());

        let quadkey = get_quadkey(latlng.lat.deg(), latlng.lng.deg(), 12);
        let quadkey_u32: u32 = quadkey.try_into().expect("Quadkey overflow");

        for keyword in &poi.content {
            if !self.keyword_index.contains_key(keyword) {
                self.keyword_index
                    .insert(keyword.clone(), RoaringBitmap::new());
            }
            if !self.keyword_frequencies.contains_key(keyword) {
                self.keyword_frequencies.insert(keyword.clone(), 0);
            }

            if let Some(bitmap) = self.keyword_index.get_mut(keyword) {
                bitmap.insert(quadkey_u32);
            } else {
                error!("Logic error: Roaring bitmap not initialized.")
            }
            if let Some(frequency) = self.keyword_frequencies.get_mut(keyword) {
                *frequency += 1;
            } else {
                error!("Logic error: Frequency not initialized.")
            }
        }
        Ok(())
    }

    pub async fn process_vocabulary(&mut self) -> Result<(), anyhow::Error> {
        let mut vocab: Vec<String> = self.keyword_index.keys().cloned().collect();
        vocab.sort_by_cached_key(|keyword| usize::MAX - self.keyword_frequencies[keyword]);

        // Vocab is effectively one-based so the 0 index can be used as a signal to stop.
        vocab.insert(0, "".to_string());
        self.keywords = vocab;
        for (idx, keyword) in self.keywords.iter().enumerate() {
            self.keyword_indices.insert(keyword.clone(), idx);
            query!(
                "INSERT INTO vocabulary (id, word) VALUES ($1, $2);",
                idx as i32,
                keyword
            )
            .execute(&mut self.conn)
            .await?;
        }
        Ok(())
    }

    pub async fn clear_db(&mut self) -> Result<(), anyhow::Error> {
        query!("DELETE FROM poi;",).execute(&mut self.conn).await?;
        query!("DELETE FROM vocabulary;",)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    pub async fn insert_poi(&mut self, poi: &SchemafiedPoi) -> Result<(), anyhow::Error> {
        let cell = Cell::from(CellID(poi.s2cell));
        let latlng = LatLng::from(cell.center());
        let quadkey = get_quadkey(latlng.lat.deg(), latlng.lng.deg(), 12);
        let quadkey_u32: u32 = quadkey.try_into().expect("Quadkey overflow");
        let keyword_indices: Vec<i32> = poi
            .content
            .iter()
            .map(|keyword| self.keyword_indices[keyword].try_into().unwrap())
            .collect();

        query!(
            "INSERT INTO poi (s2cell, quadkey, keywords) VALUES ($1, $2, $3);",
            poi.s2cell as i64,
            quadkey_u32 as i32,
            &keyword_indices
        )
        .execute(&mut self.conn)
        .await?;

        Ok(())
    }

    pub fn get_vocab(&self) -> Vec<String> {
        self.keywords.clone()
    }
}
