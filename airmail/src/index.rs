use std::{
    collections::{HashMap, HashSet},
    io::{self, Cursor},
};

use log::error;
use roaring::RoaringBitmap;
use zeekstd::EncodeOptions;

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
    // pool: PgPool,
    keyword_index: HashMap<String, RoaringBitmap>,
}

impl AirmailIndexBuilder {
    pub async fn create(url: &str) -> Result<AirmailIndexBuilder, anyhow::Error> {
        // let options = sqlx::postgres::PgConnectOptions::from_str(url)?;
        // let pool = PgPool::connect_with(options).await?;

        // let m = Migrator::new(Path::new("./airmail/migrations")).await?;
        // m.run(&pool).await?;

        // sqlx::query!("TRUNCATE poi CASCADE").execute(&pool).await?;

        Ok(AirmailIndexBuilder {
            // pool,
            keyword_index: HashMap::new(),
        })
    }

    pub async fn insert_poi(&mut self, poi: &SchemafiedPoi) -> Result<(), anyhow::Error> {
        let mut keywords = HashSet::new();
        keywords.extend(poi.content.iter().cloned());
        let indexed_keys = [
            "natural", "amenity", "shop", "leisure", "tourism", "historic", "cuisine",
        ];
        let indexed_key_prefixes = ["diet:"];
        for (key, value) in &poi.tags {
            if indexed_keys.contains(&key.as_str())
                || indexed_key_prefixes
                    .iter()
                    .any(|prefix| key.starts_with(prefix))
            {
                keywords.insert(format!("{key}={value}"));
            }
        }
        let cell = Cell::from(CellID(poi.s2cell));
        let latlng = LatLng::from(cell.center());

        for keyword in &keywords {
            if !self.keyword_index.contains_key(keyword) {
                self.keyword_index
                    .insert(keyword.clone(), RoaringBitmap::new());
            }

            let quadkey = get_quadkey(latlng.lat.deg(), latlng.lng.deg(), 12);
            let quadkey_u32: u32 = quadkey.try_into().expect("Quadkey overflow");
            // for cell in &s2_cells {
            if let Some(bitmap) = self.keyword_index.get_mut(keyword) {
                bitmap.insert(quadkey_u32);
            } else {
                error!("Logic error: Roaring bitmap not initialized.")
            }
            // }
        }

        // let mut txn = self.pool.begin().await?;
        // let mut keyword_index_new_entries = HashMap::new();

        // let mut keywords = HashSet::new();
        // keywords.extend(poi.content.iter().cloned());
        // for (key, value) in &poi.tags {
        //     keywords.insert(format!("{key}={value}"));
        // }
        // let mut s2_cells: Vec<i64> = poi.s2cell_parents.iter().map(|cell| *cell as i64).collect();
        // s2_cells.push(poi.s2cell as i64);

        // // Create a POI
        // let poi_id: i32 = sqlx::query!("INSERT INTO poi DEFAULT VALUES RETURNING id")
        //     .fetch_one(&mut *txn)
        //     .await?
        //     .id;

        // // Ensure the existence of its S2 cells and link them
        // for cell_id in s2_cells {
        //     // Link POI to S2 cell
        //     sqlx::query!(
        //         "INSERT INTO poi_s2_cells (poi_id, cell_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        //         poi_id,
        //         cell_id
        //     )
        //     .execute(&mut *txn)
        //     .await?;
        // }

        // // Ensure the existence of its keywords and link them
        // for keyword in keywords {
        //     // Insert keyword if it doesn't exist
        //     // sqlx::query!(
        //     //     "INSERT INTO keywords (word) VALUES ($1) ON CONFLICT DO NOTHING",
        //     //     keyword
        //     // )
        //     // .execute(&mut *txn)
        //     // .await?;

        //     let keyword_id = {
        //         let mut keyword_index = self.keyword_index;
        //         if let Some(id) = keyword_index.1.get(&keyword) {
        //             *id
        //         } else {
        //             // Get the keyword ID
        //             // let keyword_id: i32 =
        //             //     sqlx::query!("SELECT id FROM keywords WHERE word = $1", keyword)
        //             //         .fetch_one(&mut *txn)
        //             //         .await?
        //             //         .id;
        //             let keyword_id = keyword_index.0;
        //             keyword_index.0 += 1;
        //             keyword_index_new_entries.insert(keyword.clone(), keyword_id);
        //             keyword_id
        //         }
        //     };

        //     // Link POI to keyword
        //     sqlx::query!(
        //         "INSERT INTO poi_keywords (poi_id, keyword_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        //         poi_id,
        //         keyword_id
        //     )
        //     .execute(&mut *txn)
        //     .await?;
        // }
        // txn.commit().await?;

        Ok(())
    }

    pub async fn collect_keyword_set(&self) {
        let mut buf = Vec::new();
        for (_, bitmap) in &self.keyword_index {
            bitmap
                .serialize_into(&mut buf)
                .expect("Failed to serialize");
        }

        let compressed = Vec::new();

        let mut encoder = EncodeOptions::new()
            .frame_size_policy(zeekstd::FrameSizePolicy::Uncompressed(10_000_000))
            .compression_level(22)
            .into_encoder(compressed)
            .unwrap();
        let mut cursor = Cursor::new(buf);
        io::copy(&mut cursor, &mut encoder).unwrap();
        // End compression and write the seek table to the end of the seekable

        dbg!(encoder.finish().unwrap());
        dbg!(self
            .keyword_index
            .iter()
            .map(|(_keyword, bitmap)| bitmap.serialized_size())
            .sum::<usize>());
        dbg!(self.keyword_index.iter().count());

        let mut builder = fst::SetBuilder::memory();
        let mut all_keywords: Vec<String> = self.keyword_index.keys().cloned().collect();
        all_keywords.sort();
        builder.extend_iter(all_keywords.iter()).unwrap();
        let fst = builder.into_inner().unwrap();
        dbg!(fst.len());

        let compressed_fst = Vec::new();

        let mut encoder = EncodeOptions::new()
            .frame_size_policy(zeekstd::FrameSizePolicy::Uncompressed(10_000_000))
            .compression_level(22)
            .into_encoder(compressed_fst)
            .unwrap();
        let mut cursor = Cursor::new(fst);
        io::copy(&mut cursor, &mut encoder).unwrap();
        // End compression and write the seek table to the end of the seekable

        dbg!(encoder.finish().unwrap());
    }
}
