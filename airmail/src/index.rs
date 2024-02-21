use std::sync::Arc;

use airmail_common::categories::PoiCategory;
use futures_util::future::join_all;
use serde_json::Value;
use tantivy::{
    collector::{Count, TopDocs},
    directory::MmapDirectory,
    query::{BooleanQuery, BoostQuery, PhraseQuery, Query, TermQuery},
    schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions, FAST, INDEXED, STORED},
    Document, Term,
};
use tokio::task::spawn_blocking;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    directory::HttpDirectory,
    poi::{AirmailPoi, ToIndexPoi},
    query::all_subsequences,
};

// Field name keys.
pub const FIELD_CONTENT: &str = "content";
pub const FIELD_SOURCE: &str = "source";
pub const FIELD_S2CELL: &str = "s2cell";
pub const FIELD_TAGS: &str = "tags";

#[derive(Clone)]
pub struct AirmailIndex {
    tantivy_index: Arc<tantivy::Index>,
}

impl AirmailIndex {
    fn schema() -> tantivy::schema::Schema {
        let mut schema_builder = Schema::builder();
        let text_options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_fieldnorms(false)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );

        let _ = schema_builder.add_text_field(FIELD_CONTENT, text_options.clone());
        let _ = schema_builder.add_text_field(FIELD_SOURCE, text_options.clone());
        let _ = schema_builder.add_u64_field(FIELD_S2CELL, INDEXED | STORED | FAST);
        let _ = schema_builder.add_json_field(FIELD_TAGS, STORED);
        schema_builder.build()
    }

    pub fn field_content(&self) -> tantivy::schema::Field {
        self.tantivy_index
            .schema()
            .get_field(FIELD_CONTENT)
            .unwrap()
    }

    pub fn field_source(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_SOURCE).unwrap()
    }

    pub fn field_s2cell(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_S2CELL).unwrap()
    }

    pub fn field_tags(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_TAGS).unwrap()
    }

    pub fn create(index_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema = Self::schema();
        let tantivy_index =
            tantivy::Index::open_or_create(MmapDirectory::open(index_dir)?, schema)?;
        Ok(Self {
            tantivy_index: Arc::new(tantivy_index),
        })
    }

    pub fn new(index_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tantivy_index = tantivy::Index::open_in_dir(index_dir)?;
        Ok(Self {
            tantivy_index: Arc::new(tantivy_index),
        })
    }

    pub fn new_remote(base_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tantivy_index = tantivy::Index::open(HttpDirectory::new(base_url))?;
        Ok(Self {
            tantivy_index: Arc::new(tantivy_index),
        })
    }

    pub fn writer(&mut self) -> Result<AirmailIndexWriter, Box<dyn std::error::Error>> {
        let tantivy_writer = self.tantivy_index.writer(200_000_000)?;
        let writer = AirmailIndexWriter {
            tantivy_writer,
            schema: self.tantivy_index.schema(),
        };
        Ok(writer)
    }

    pub async fn merge(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let ids = self.tantivy_index.searchable_segment_ids()?;
        self.tantivy_index.writer(200_000_000)?.merge(&ids).await?;
        Ok(())
    }

    pub async fn num_docs(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let index = self.tantivy_index.clone();
        let count = spawn_blocking(move || {
            if let Ok(tantivy_reader) = index.reader() {
                Some(tantivy_reader.searcher().num_docs())
            } else {
                None
            }
        });
        Ok(count.await?.ok_or("Error getting count")?)
    }

    pub async fn search(
        &self,
        query: &str,
    ) -> Result<Vec<(AirmailPoi, f32)>, Box<dyn std::error::Error>> {
        let tantivy_reader = self.tantivy_index.reader()?;
        let searcher = tantivy_reader.searcher();
        let mut queries: Vec<Box<dyn Query>> = Vec::new();

        let query = query.trim().replace("'s", "s");

        {
            let tokens: Vec<String> = query
                .split_word_bounds()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            for subsequence in all_subsequences(&tokens) {
                // if tokens.len() > 3 && subsequence.len() == 2 {
                //     continue;
                // }

                let possible_query = subsequence.join(" ");
                let non_alphabetic = possible_query
                    .chars()
                    .filter(|c| c.is_numeric() || c.is_whitespace())
                    .count();
                let total_chars = possible_query.chars().count();
                if total_chars < 3 && non_alphabetic == 0 {
                    continue;
                }
                let term = Term::from_field_text(self.field_content(), &possible_query);
                let mut boost = 1.05f32.powf(possible_query.len() as f32);
                // Anecdotally, numbers in queries are usually important.
                if total_chars - non_alphabetic < 3 && non_alphabetic > 0 {
                    boost *= 3.0;
                }
                if subsequence.len() > 1 {
                    {
                        let searcher = searcher.clone();
                        let subsequence = subsequence.clone();
                        let content_field = self.field_content();
                        spawn_blocking(move || {
                            let _ = searcher.search(
                                &PhraseQuery::new(
                                    subsequence
                                        .iter()
                                        .map(|s| Term::from_field_text(content_field, s))
                                        .collect(),
                                ),
                                &Count,
                            );
                        });
                    }

                    queries.push(Box::new(BoostQuery::new(
                        Box::new(PhraseQuery::new(
                            subsequence
                                .iter()
                                .map(|s| Term::from_field_text(self.field_content(), s))
                                .collect(),
                        )),
                        boost,
                    )));
                } else {
                    {
                        let searcher = searcher.clone();
                        let term = term.clone();
                        spawn_blocking(move || {
                            let _ = searcher
                                .search(&TermQuery::new(term, IndexRecordOption::Basic), &Count);
                        });
                    }
                    queries.push(Box::new(BoostQuery::new(
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                        boost,
                    )));
                }
            }
        }

        let query = BooleanQuery::union(queries);
        let (top_docs, searcher) =
            spawn_blocking(move || (searcher.search(&query, &TopDocs::with_limit(10)), searcher))
                .await?;
        let mut scores = Vec::new();
        let mut futures = Vec::new();
        for (score, doc_id) in top_docs? {
            let searcher = searcher.clone();
            let doc = spawn_blocking(move || searcher.doc(doc_id));
            scores.push(score);
            futures.push(doc);
        }
        let mut results = Vec::new();
        let top_docs = join_all(futures).await;
        for (score, doc_future) in scores.iter().zip(top_docs) {
            let doc = doc_future??;
            let s2cell = doc
                .get_first(self.field_s2cell())
                .unwrap()
                .as_u64()
                .unwrap();
            let cellid = s2::cellid::CellID(s2cell);
            let latlng = s2::latlng::LatLng::from(cellid);
            let tags: Vec<(String, String)> = doc
                .get_first(self.field_tags())
                .map(|v| v.as_json().unwrap())
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                .collect();

            let poi = AirmailPoi::new(
                vec![],
                "".to_string(),
                PoiCategory::Address, // FIXME.
                vec![],
                vec![],
                vec![],
                latlng.lat.deg(),
                latlng.lng.deg(),
                tags,
            )?;
            results.push((poi, *score));
        }

        Ok(results)
    }
}

pub struct AirmailIndexWriter {
    tantivy_writer: tantivy::IndexWriter,
    schema: Schema,
}

impl AirmailIndexWriter {
    fn process_field(&self, doc: &mut Document, value: &str) {
        doc.add_text(self.schema.get_field(FIELD_CONTENT).unwrap(), value);
    }

    pub async fn add_poi(&mut self, poi: ToIndexPoi) -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = tantivy::Document::default();
        for content in poi.content {
            self.process_field(&mut doc, &content);
        }
        doc.add_text(self.schema.get_field(FIELD_SOURCE).unwrap(), poi.source);

        doc.add_json_object(
            self.schema.get_field(FIELD_TAGS).unwrap(),
            poi.tags
                .iter()
                .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
                .collect::<serde_json::Map<String, Value>>(),
        );
        doc.add_u64(self.schema.get_field(FIELD_S2CELL).unwrap(), poi.s2cell);
        self.tantivy_writer.add_document(doc)?;

        Ok(())
    }

    pub fn commit(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.tantivy_writer.commit()?;
        Ok(())
    }
}
