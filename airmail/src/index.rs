use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use futures_util::future::join_all;
use geo::Rect;
use itertools::Itertools;
use log::debug;
use s2::region::RegionCoverer;
use std::collections::BTreeMap;
use tantivy::schema::Value;
use tantivy::{
    collector::{Count, TopDocs},
    directory::MmapDirectory,
    query::{
        BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, PhrasePrefixQuery, PhraseQuery, Query,
        TermQuery,
    },
    schema::{
        IndexRecordOption, NumericOptions, OwnedValue, Schema, TextFieldIndexing, TextOptions,
        STORED,
    },
    Searcher, TantivyDocument, Term,
};
use tantivy_uffd::RemoteDirectory;
use tokio::task::spawn_blocking;
use unicode_segmentation::UnicodeSegmentation;

use crate::error::AirmailError;
use crate::{
    poi::{AirmailPoi, SchemafiedPoi},
    query::all_subsequences,
};

// Field name keys.
pub const FIELD_CONTENT: &str = "content";
pub const FIELD_INDEXED_TAG: &str = "indexed_tag";
pub const FIELD_SOURCE: &str = "source";
pub const FIELD_S2CELL: &str = "s2cell";
pub const FIELD_S2CELL_PARENTS: &str = "s2cell_parents";
pub const FIELD_CATEGORY_JSON: &str = "category";
pub const FIELD_TAGS: &str = "tags";

#[derive(Clone)]
pub struct AirmailIndex {
    tantivy_index: Arc<tantivy::Index>,
    is_remote: bool,
}

impl AirmailIndex {
    fn schema() -> tantivy::schema::Schema {
        let mut schema_builder = Schema::builder();
        let text_options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_fieldnorms(false)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );
        let tag_options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_fieldnorms(false)
                .set_tokenizer("raw")
                .set_index_option(IndexRecordOption::Basic),
        );
        let s2cell_parent_index_options = NumericOptions::default().set_indexed();
        let s2cell_index_options = NumericOptions::default()
            .set_indexed()
            .set_stored()
            .set_fast();
        assert!(!s2cell_parent_index_options.fieldnorms());
        assert!(!s2cell_index_options.fieldnorms());

        let _ = schema_builder.add_text_field(FIELD_CONTENT, text_options.clone());
        let _ = schema_builder.add_text_field(FIELD_INDEXED_TAG, tag_options);
        let _ = schema_builder.add_text_field(FIELD_SOURCE, text_options.clone());
        let _ = schema_builder.add_u64_field(FIELD_S2CELL, s2cell_index_options);
        let _ = schema_builder.add_u64_field(FIELD_S2CELL_PARENTS, s2cell_parent_index_options);
        let _ = schema_builder.add_json_field(FIELD_TAGS, STORED);
        let _ = schema_builder.add_text_field(FIELD_CATEGORY_JSON, STORED);
        schema_builder.build()
    }

    fn field_content(&self) -> tantivy::schema::Field {
        self.tantivy_index
            .schema()
            .get_field(FIELD_CONTENT)
            .unwrap()
    }

    fn field_indexed_tag(&self) -> tantivy::schema::Field {
        self.tantivy_index
            .schema()
            .get_field(FIELD_INDEXED_TAG)
            .unwrap()
    }

    fn field_source(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_SOURCE).unwrap()
    }

    fn field_s2cell(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_S2CELL).unwrap()
    }

    fn field_s2cell_parents(&self) -> tantivy::schema::Field {
        self.tantivy_index
            .schema()
            .get_field(FIELD_S2CELL_PARENTS)
            .unwrap()
    }

    fn field_tags(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_TAGS).unwrap()
    }

    pub fn create(index_dir: &PathBuf) -> Result<Self> {
        let schema = Self::schema();
        let tantivy_index =
            tantivy::Index::open_or_create(MmapDirectory::open(index_dir)?, schema)?;
        Ok(Self {
            tantivy_index: Arc::new(tantivy_index),
            is_remote: false,
        })
    }

    pub fn new(index_dir: &str) -> Result<Self> {
        let tantivy_index = tantivy::Index::open_in_dir(index_dir)?;
        Ok(Self {
            tantivy_index: Arc::new(tantivy_index),
            is_remote: false,
        })
    }

    pub fn new_remote(base_url: &str) -> Result<Self> {
        let tantivy_index =
            tantivy::Index::open(RemoteDirectory::<{ 2 * 1024 * 1024 }>::new(base_url))?;
        Ok(Self {
            tantivy_index: Arc::new(tantivy_index),
            is_remote: true,
        })
    }

    pub fn writer(&mut self) -> Result<AirmailIndexWriter> {
        let tantivy_writer = self
            .tantivy_index
            .writer::<TantivyDocument>(2_000_000_000)?;
        let writer = AirmailIndexWriter {
            tantivy_writer,
            schema: self.tantivy_index.schema(),
        };
        Ok(writer)
    }

    pub async fn merge(&mut self) -> Result<()> {
        let ids = self.tantivy_index.searchable_segment_ids()?;
        self.tantivy_index
            .writer::<TantivyDocument>(2_000_000_000)?
            .merge(&ids)
            .await?;
        Ok(())
    }

    pub async fn num_docs(&self) -> Result<u64> {
        let index = self.tantivy_index.clone();
        let count = spawn_blocking(move || {
            if let Ok(tantivy_reader) = index.reader() {
                Some(tantivy_reader.searcher().num_docs())
            } else {
                None
            }
        });
        Ok(count.await?.ok_or(AirmailError::UnableToCount)?)
    }

    async fn construct_query(
        &self,
        searcher: &Searcher,
        query: &str,
        tags: Option<Vec<String>>,
        bbox: Option<Rect<f64>>,
        _boost_regions: &[(f32, Rect<f64>)],
        lenient: bool,
    ) -> Box<dyn Query> {
        let mut queries: Vec<Box<dyn Query>> = Vec::new();
        let mut mandatory_queries: Vec<Box<dyn Query>> = Vec::new();

        let tokens: Vec<String> = query
            .split_word_bounds()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for subsequence in all_subsequences(&tokens) {
            let possible_query = subsequence.join(" ");
            if possible_query
                .chars()
                .all(|c| c.is_whitespace() || c.is_ascii_punctuation())
            {
                continue;
            }

            let non_alphabetic = possible_query
                .chars()
                .filter(|c| c.is_numeric() || c.is_whitespace())
                .count();
            let total_chars = possible_query.chars().count();
            let term = Term::from_field_text(self.field_content(), &possible_query);
            let mut boost = 1.05f32.powf(possible_query.len() as f32);
            // Anecdotally, numbers in queries are usually important.
            if total_chars - non_alphabetic < 3 && non_alphabetic > 0 {
                boost *= 3.0;
            }
            if subsequence.len() > 1 {
                if self.is_remote {
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

                if self.is_remote {
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
                    queries.push(Box::new(BoostQuery::new(
                        Box::new(PhrasePrefixQuery::new(
                            subsequence
                                .iter()
                                .map(|s| Term::from_field_text(self.field_content(), s))
                                .collect(),
                        )),
                        boost,
                    )));
                }
            } else if possible_query.len() >= 8 && lenient {
                let query = if tokens.ends_with(&[possible_query]) {
                    FuzzyTermQuery::new_prefix(term, 1, true)
                } else {
                    FuzzyTermQuery::new(term, 1, true)
                };
                if self.is_remote {
                    let searcher = searcher.clone();
                    let query = query.clone();
                    spawn_blocking(move || {
                        let _ = searcher.search(&query, &Count);
                    });
                }
                mandatory_queries.push(Box::new(BoostQuery::new(Box::new(query), boost)));
            } else {
                let query: Box<dyn Query> =
                    if self.is_remote || !lenient || !tokens.ends_with(&[possible_query]) {
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic))
                    } else {
                        Box::new(FuzzyTermQuery::new_prefix(term, 0, false))
                    };
                if self.is_remote {
                    let searcher = searcher.clone();
                    let query = query.box_clone();
                    spawn_blocking(move || {
                        let _ = searcher.search(&query, &Count);
                    });
                }
                mandatory_queries.push(Box::new(BoostQuery::new(query, boost)));
            }
        }

        if let Some(tags) = tags {
            for tag in &tags {
                let term = Term::from_field_text(self.field_indexed_tag(), tag);
                let query: Box<dyn Query> =
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                mandatory_queries.push(query);
            }
        }

        let optional = BooleanQuery::union(queries);
        let required = BooleanQuery::intersection(mandatory_queries);
        let final_query = BooleanQuery::new(vec![
            (Occur::Should, Box::new(optional)),
            (Occur::Must, Box::new(required)),
        ]);

        if let Some(bbox) = bbox {
            let region = s2::rect::Rect::from_degrees(
                bbox.min().y,
                bbox.min().x,
                bbox.max().y,
                bbox.max().x,
            );
            let covering_cells = {
                let coverer = RegionCoverer {
                    min_level: 0,
                    max_level: 16,
                    level_mod: 1,
                    max_cells: 64,
                };
                let mut cellunion = coverer.covering(&region);
                cellunion.normalize();
                cellunion.0.iter().map(|c| c.0).collect_vec()
            };
            let covering_disjunction_clauses = covering_cells
                .iter()
                .map(|c| {
                    let term = Term::from_field_u64(self.field_s2cell_parents(), *c);
                    let query: Box<dyn Query> =
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                    query
                })
                .collect_vec();
            let covering_query = BooleanQuery::union(covering_disjunction_clauses);
            return Box::new(BooleanQuery::intersection(vec![
                Box::new(covering_query),
                Box::new(final_query),
            ]));
        }

        Box::new(final_query)
    }

    /// This is public because I don't want one big mega-crate but its API should not be considered even remotely stable.
    pub async fn search(
        &self,
        query: &str,
        request_leniency: bool,
        tags: Option<Vec<String>>,
        bbox: Option<Rect<f64>>,
        boost_regions: &[(f32, Rect<f64>)],
    ) -> Result<Vec<(AirmailPoi, f32)>> {
        let tantivy_reader = self.tantivy_index.reader()?;
        let searcher = tantivy_reader.searcher();
        let query_string = query.trim().replace("'s", "s");

        let start = std::time::Instant::now();
        let (top_docs, searcher) = {
            let query = self
                .construct_query(
                    &searcher,
                    &query_string,
                    tags,
                    bbox,
                    boost_regions,
                    request_leniency,
                )
                .await;

            #[cfg(feature = "invasive_logging")]
            {
                dbg!(&query);
            }

            let (top_docs, searcher) = spawn_blocking(move || {
                (searcher.search(&query, &TopDocs::with_limit(10)), searcher)
            })
            .await?;
            let top_docs = top_docs?;
            debug!(
                "Search took {:?} and yielded {} results",
                start.elapsed(),
                top_docs.len()
            );
            (top_docs, searcher)
        };

        let mut scores = Vec::new();
        let mut futures = Vec::new();
        for (score, doc_id) in top_docs {
            let searcher = searcher.clone();
            let doc = spawn_blocking(move || searcher.doc::<TantivyDocument>(doc_id));
            scores.push(score);
            futures.push(doc);
        }
        let mut results = Vec::new();
        let top_docs = join_all(futures).await;
        for (score, doc_future) in scores.iter().zip(top_docs) {
            let doc = doc_future??;
            let source = doc
                .get_first(self.field_source())
                .map(|value| value.as_str().unwrap().to_string())
                .unwrap_or_default();
            let s2cell = doc
                .get_first(self.field_s2cell())
                .unwrap()
                .as_u64()
                .unwrap();
            let cellid = s2::cellid::CellID(s2cell);
            let latlng = s2::latlng::LatLng::from(cellid);
            let tags: Vec<(String, String)> = doc
                .get_first(self.field_tags())
                .unwrap()
                .as_object()
                .unwrap()
                .map(|(k, v)| (k.to_string(), v.as_str().unwrap().to_string()))
                .collect();

            let poi = AirmailPoi::new(source, latlng.lat.deg(), latlng.lng.deg(), tags)?;
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
    fn process_field(&self, doc: &mut TantivyDocument, value: &str) {
        doc.add_text(self.schema.get_field(FIELD_CONTENT).unwrap(), value);
    }

    pub async fn add_poi(&mut self, poi: SchemafiedPoi, source: &str) -> Result<()> {
        let mut doc = TantivyDocument::default();
        for content in poi.content {
            self.process_field(&mut doc, &content);
        }
        doc.add_text(self.schema.get_field(FIELD_SOURCE).unwrap(), source);

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
                doc.add_text(
                    self.schema.get_field(FIELD_INDEXED_TAG).unwrap(),
                    format!("{}={}", key, value).as_str(),
                );
            }
        }
        doc.add_object(
            self.schema.get_field(FIELD_TAGS).unwrap(),
            poi.tags
                .iter()
                .map(|(k, v)| (k.to_string(), OwnedValue::Str(v.to_string())))
                .collect::<BTreeMap<String, OwnedValue>>(),
        );

        doc.add_u64(self.schema.get_field(FIELD_S2CELL).unwrap(), poi.s2cell);
        for parent in poi.s2cell_parents {
            doc.add_u64(self.schema.get_field(FIELD_S2CELL_PARENTS).unwrap(), parent);
        }
        self.tantivy_writer.add_document(doc)?;

        Ok(())
    }

    pub fn commit(mut self) -> Result<()> {
        self.tantivy_writer.commit()?;
        Ok(())
    }
}
