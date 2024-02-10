use airmail_parser::{component::QueryComponentType, query::QueryScenario};
use serde_json::Value;
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::{BooleanQuery, DisjunctionMaxQuery, FuzzyTermQuery, Query},
    schema::{FacetOptions, Schema, TextFieldIndexing, TextOptions, INDEXED, STORED, TEXT},
    tokenizer::{LowerCaser, RawTokenizer, TextAnalyzer},
    Term,
};

use crate::{directory::HttpDirectory, poi::AirmailPoi};

// Field name keys.
pub const FIELD_NAME: &str = "name";
pub const FIELD_SOURCE: &str = "source";
pub const FIELD_CATEGORY: &str = "category";
pub const FIELD_HOUSE_NUMBER: &str = "house_number";
pub const FIELD_ROAD: &str = "road";
pub const FIELD_UNIT: &str = "unit";
pub const FIELD_LOCALITY: &str = "locality";
pub const FIELD_REGION: &str = "region";
pub const FIELD_COUNTRY: &str = "country";
pub const FIELD_S2CELL: &str = "s2cell";
pub const FIELD_TAGS: &str = "tags";

pub struct AirmailIndex {
    tantivy_index: tantivy::Index,
}

fn query_for_terms(
    field: tantivy::schema::Field,
    terms: Vec<&str>,
    is_prefix: bool,
) -> Result<Vec<Box<dyn Query>>, Box<dyn std::error::Error>> {
    let mut queries: Vec<Box<dyn Query>> = Vec::new();
    let mut phrase = Vec::new();
    for (i, term) in terms.iter().enumerate() {
        if term.len() < 2 {
            continue;
        }
        phrase.push(Term::from_field_text(field, term));
        if i == terms.len() - 1 && is_prefix {
            queries.push(Box::new(FuzzyTermQuery::new_prefix(
                Term::from_field_text(field, term),
                0,
                true,
            )));
        } else {
            queries.push(Box::new(FuzzyTermQuery::new(
                Term::from_field_text(field, term),
                0,
                true,
            )));
        };
    }
    let queries: Vec<Box<dyn Query>> = vec![Box::new(BooleanQuery::intersection(queries))];
    Ok(queries)
}

impl AirmailIndex {
    fn schema() -> tantivy::schema::Schema {
        let mut schema_builder = Schema::builder();
        let street_options = TextOptions::default()
            .set_stored()
            .set_indexing_options(TextFieldIndexing::default().set_tokenizer("street_tokenizer"));

        let _ = schema_builder.add_text_field(FIELD_NAME, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_SOURCE, TEXT | STORED);
        let _ =
            schema_builder.add_facet_field(FIELD_CATEGORY, FacetOptions::default().set_stored());
        let _ = schema_builder.add_text_field(FIELD_HOUSE_NUMBER, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_ROAD, street_options);
        let _ = schema_builder.add_text_field(FIELD_UNIT, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_LOCALITY, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_REGION, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_COUNTRY, TEXT | STORED);
        let _ = schema_builder.add_u64_field(FIELD_S2CELL, INDEXED | STORED);
        let _ = schema_builder.add_json_field(FIELD_TAGS, STORED);
        schema_builder.build()
    }

    pub fn field_name(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_NAME).unwrap()
    }

    pub fn field_source(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_SOURCE).unwrap()
    }

    pub fn field_house_number(&self) -> tantivy::schema::Field {
        self.tantivy_index
            .schema()
            .get_field(FIELD_HOUSE_NUMBER)
            .unwrap()
    }

    pub fn field_road(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_ROAD).unwrap()
    }

    pub fn field_unit(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_UNIT).unwrap()
    }

    pub fn field_locality(&self) -> tantivy::schema::Field {
        self.tantivy_index
            .schema()
            .get_field(FIELD_LOCALITY)
            .unwrap()
    }

    pub fn field_region(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_REGION).unwrap()
    }

    pub fn field_country(&self) -> tantivy::schema::Field {
        self.tantivy_index
            .schema()
            .get_field(FIELD_COUNTRY)
            .unwrap()
    }

    pub fn field_s2cell(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_S2CELL).unwrap()
    }

    pub fn field_tags(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_TAGS).unwrap()
    }

    pub fn create(index_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema = Self::schema();
        let street_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
            .filter(LowerCaser)
            .build();
        let tantivy_index =
            tantivy::Index::open_or_create(MmapDirectory::open(index_dir)?, schema)?;
        tantivy_index
            .tokenizers()
            .register("street_tokenizer", street_tokenizer);
        Ok(Self { tantivy_index })
    }

    pub fn new(index_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let street_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
            .filter(LowerCaser)
            .build();
        let tantivy_index = tantivy::Index::open_in_dir(index_dir)?;
        tantivy_index
            .tokenizers()
            .register("street_tokenizer", street_tokenizer);
        Ok(Self { tantivy_index })
    }

    pub fn new_remote(base_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let street_tokenizer = TextAnalyzer::builder(RawTokenizer::default())
            .filter(LowerCaser)
            .build();
        let tantivy_index = tantivy::Index::open(HttpDirectory::new(base_url))?;
        tantivy_index
            .tokenizers()
            .register("street_tokenizer", street_tokenizer);
        Ok(Self { tantivy_index })
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

    pub fn search(
        &self,
        query: &QueryScenario,
    ) -> Result<Vec<(AirmailPoi, f32)>, Box<dyn std::error::Error>> {
        let tantivy_reader = self.tantivy_index.reader()?;
        let searcher = tantivy_reader.searcher();
        let mut queries: Vec<Box<dyn Query>> = Vec::new();
        let query_vec = query.as_vec();
        for (i, component) in query_vec.iter().enumerate() {
            let is_prefix = i == query_vec.len() - 1;
            let terms: Vec<String> = component
                .text()
                .split_whitespace()
                .map(|term| term.to_lowercase())
                .collect();
            let term_strs = terms.iter().map(|s| s.as_str()).collect();
            if terms.is_empty() {
                continue;
            }
            match component.component_type() {
                QueryComponentType::NearComponent => {
                    // No-op
                }

                QueryComponentType::HouseNumberComponent => {
                    queries.extend(query_for_terms(
                        self.field_house_number(),
                        term_strs,
                        is_prefix,
                    )?);
                }

                QueryComponentType::RoadComponent => {
                    if is_prefix {
                        queries.push(Box::new(FuzzyTermQuery::new_prefix(
                            Term::from_field_text(self.field_road(), component.text()),
                            0,
                            true,
                        )));
                    } else {
                        queries.push(Box::new(FuzzyTermQuery::new(
                            Term::from_field_text(self.field_road(), component.text()),
                            0,
                            true,
                        )));
                    }
                }

                QueryComponentType::IntersectionComponent => {
                    // No-op
                }

                QueryComponentType::SublocalityComponent => {
                    // No-op, and probably always will be. "Downtown" is very subjective, for example.
                }

                QueryComponentType::LocalityComponent => {
                    queries.extend(query_for_terms(
                        self.field_locality(),
                        term_strs,
                        is_prefix,
                    )?);
                }

                QueryComponentType::RegionComponent => {
                    queries.extend(query_for_terms(self.field_region(), term_strs, is_prefix)?);
                }

                QueryComponentType::CountryComponent => {
                    queries.extend(query_for_terms(self.field_country(), term_strs, is_prefix)?);
                }

                QueryComponentType::CategoryComponent | QueryComponentType::PlaceNameComponent => {
                    let mut new_terms = Vec::new();
                    for term in &terms {
                        if term.ends_with("'s") {
                            new_terms.push(term.trim_end_matches("'s"));
                        }
                    }
                    let original = query_for_terms(self.field_name(), term_strs, is_prefix)?;
                    queries.extend(original);
                    if !new_terms.is_empty() {
                        let modified = query_for_terms(self.field_name(), new_terms, false)?;
                        queries.extend(modified);
                    }
                }

                QueryComponentType::IntersectionJoinWordComponent => {
                    // No-op
                }
            }
        }

        let query = DisjunctionMaxQuery::with_tie_breaker(queries, 5.0);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            let house_num: Option<&str> = doc
                .get_first(self.field_house_number())
                .map(|v| v.as_text())
                .flatten();
            let road: Option<&str> = doc
                .get_first(self.field_road())
                .map(|v| v.as_text())
                .flatten();
            let unit: Option<&str> = doc
                .get_first(self.field_unit())
                .map(|v| v.as_text())
                .flatten();
            let locality: Vec<&str> = doc
                .get_all(self.field_locality())
                .filter_map(|v| v.as_text())
                .collect();
            let region: Option<&str> = doc
                .get_first(self.field_region())
                .map(|v| v.as_text())
                .flatten();
            let country: Option<&str> = doc
                .get_first(self.field_country())
                .map(|v| v.as_text())
                .flatten();
            let s2cell = doc
                .get_first(self.field_s2cell())
                .unwrap()
                .as_u64()
                .unwrap();
            let cellid = s2::cellid::CellID(s2cell);
            let latlng = s2::latlng::LatLng::from(cellid);
            let names = doc
                .get_all(self.field_name())
                .filter_map(|v| v.as_text())
                .collect::<Vec<&str>>();
            let tags: Vec<(String, String)> = doc
                .get_first(self.field_tags())
                .map(|v| v.as_json().unwrap())
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                .collect();

            let mut poi = AirmailPoi::new(
                names.iter().map(|s| s.to_string()).collect(),
                doc.get_first(self.field_source())
                    .unwrap()
                    .as_text()
                    .unwrap()
                    .to_string(),
                vec![], // FIXME
                house_num.iter().map(|s| s.to_string()).collect(),
                road.iter().map(|s| s.to_string()).collect(),
                unit.iter().map(|s| s.to_string()).collect(),
                latlng.lat.deg(),
                latlng.lng.deg(),
                tags,
            )?;
            poi.locality = locality.iter().map(|s| s.to_string()).collect();
            poi.region = region.map(|s| s.to_string()).into_iter().collect();
            poi.country = country.map(|s| s.to_string()).into_iter().collect();
            results.push((poi, score));
        }

        Ok(results)
    }
}

pub struct AirmailIndexWriter {
    tantivy_writer: tantivy::IndexWriter,
    schema: Schema,
}

impl AirmailIndexWriter {
    pub fn add_poi(&mut self, poi: AirmailPoi) -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = tantivy::Document::default();
        for name in poi.name {
            doc.add_text(self.schema.get_field(FIELD_NAME).unwrap(), name);
        }
        doc.add_text(self.schema.get_field(FIELD_SOURCE).unwrap(), poi.source);
        for category in poi.category {
            doc.add_facet(self.schema.get_field(FIELD_CATEGORY).unwrap(), &category);
        }
        for house_number in poi.house_number {
            doc.add_text(
                self.schema.get_field(FIELD_HOUSE_NUMBER).unwrap(),
                house_number,
            );
        }
        for road in poi.road {
            doc.add_text(self.schema.get_field(FIELD_ROAD).unwrap(), road);
        }
        for unit in poi.unit {
            doc.add_text(self.schema.get_field(FIELD_UNIT).unwrap(), unit);
        }
        for locality in poi.locality {
            doc.add_text(self.schema.get_field(FIELD_LOCALITY).unwrap(), locality);
        }
        for region in poi.region {
            doc.add_text(self.schema.get_field(FIELD_REGION).unwrap(), region);
        }
        for country in poi.country {
            doc.add_text(self.schema.get_field(FIELD_COUNTRY).unwrap(), country);
        }
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
