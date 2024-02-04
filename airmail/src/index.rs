use airmail_parser::{component::QueryComponentType, query::QueryScenario};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::{BooleanQuery, PhrasePrefixQuery, Query, TermQuery},
    schema::{IndexRecordOption, Schema, INDEXED, STORED, TEXT},
    Term,
};

use crate::poi::AirmailPoi;

// Field name keys.
pub const FIELD_NAME: &str = "name";
pub const FIELD_CATEGORY: &str = "category";
pub const FIELD_HOUSE_NUMBER: &str = "house_number";
pub const FIELD_ROAD: &str = "road";
pub const FIELD_UNIT: &str = "unit";
pub const FIELD_LOCALITY: &str = "locality";
pub const FIELD_REGION: &str = "region";
pub const FIELD_S2CELL: &str = "s2cell";

pub struct AirmailIndex {
    tantivy_index: tantivy::Index,
}

fn query_for_terms(
    field: tantivy::schema::Field,
    terms: Vec<&str>,
) -> Result<Box<dyn Query>, Box<dyn std::error::Error>> {
    if terms.len() > 1 {
        Ok(Box::new(PhrasePrefixQuery::new(
            terms
                .iter()
                .map(|token| Term::from_field_text(field, token))
                .collect(),
        )))
    } else {
        Ok(Box::new(TermQuery::new(
            Term::from_field_text(field, terms[0]),
            IndexRecordOption::Basic,
        )))
    }
}

impl AirmailIndex {
    fn schema() -> tantivy::schema::Schema {
        let mut schema_builder = Schema::builder();
        let _ = schema_builder.add_text_field(FIELD_NAME, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_CATEGORY, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_HOUSE_NUMBER, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_ROAD, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_UNIT, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_LOCALITY, TEXT | STORED);
        let _ = schema_builder.add_text_field(FIELD_REGION, TEXT | STORED);
        let _ = schema_builder.add_u64_field(FIELD_S2CELL, INDEXED | STORED);
        schema_builder.build()
    }

    pub fn field_name(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_NAME).unwrap()
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

    pub fn field_s2cell(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_S2CELL).unwrap()
    }

    pub fn field_region(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_REGION).unwrap()
    }

    // pub fn field_country(&self) -> tantivy::schema::Field {
    //     self.tantivy_index
    //         .schema()
    //         .get_field(FIELD_COUNTRY)
    //         .unwrap()
    // }

    pub fn create(index_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema = Self::schema();
        let tantivy_index =
            tantivy::Index::open_or_create(MmapDirectory::open(index_dir)?, schema)?;
        Ok(Self { tantivy_index })
    }

    pub fn new(index_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tantivy_index = tantivy::Index::open_in_dir(index_dir)?;
        Ok(Self { tantivy_index })
    }

    pub fn writer(&mut self) -> Result<AirmailIndexWriter, Box<dyn std::error::Error>> {
        let tantivy_writer = self.tantivy_index.writer(50_000_000)?;
        let writer = AirmailIndexWriter {
            tantivy_writer,
            schema: self.tantivy_index.schema(),
        };
        Ok(writer)
    }

    pub fn search(&self, query: &QueryScenario) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let tantivy_reader = self.tantivy_index.reader()?;
        let searcher = tantivy_reader.searcher();
        let mut queries: Vec<Box<dyn Query>> = Vec::new();
        for component in &query.as_vec() {
            let terms: Vec<&str> = component.text().split_whitespace().collect();
            if terms.is_empty() {
                continue;
            }
            match component.component_type() {
                QueryComponentType::CategoryComponent => {
                    // No-op
                }

                QueryComponentType::NearComponent => {
                    // No-op
                }

                QueryComponentType::HouseNumberComponent => {
                    queries.push(query_for_terms(self.field_house_number(), terms)?);
                }

                QueryComponentType::RoadComponent => {
                    queries.push(query_for_terms(self.field_road(), terms)?);
                }

                QueryComponentType::IntersectionComponent => {
                    // No-op
                }

                QueryComponentType::SublocalityComponent => {
                    // No-op
                }

                QueryComponentType::LocalityComponent => {
                    queries.push(query_for_terms(self.field_locality(), terms)?);
                }

                QueryComponentType::RegionComponent => {
                    queries.push(query_for_terms(self.field_region(), terms)?);
                }

                QueryComponentType::CountryComponent => {
                    // No-op
                }

                QueryComponentType::PlaceNameComponent => {
                    // No-op
                }

                QueryComponentType::IntersectionJoinWordComponent => {
                    // No-op
                }
            }
        }

        let query = BooleanQuery::intersection(queries);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
        println!("Found {} hits", top_docs.len());
        let results = Vec::new();
        for (_score, doc_address) in top_docs {
            let doc = searcher.doc(doc_address)?;
            let house_num: Vec<&str> = doc
                .get_all(self.field_house_number())
                .filter_map(|v| v.as_text())
                .collect();
            let road: Vec<&str> = doc
                .get_all(self.field_road())
                .filter_map(|v| v.as_text())
                .collect();
            let unit: Vec<&str> = doc
                .get_all(self.field_unit())
                .filter_map(|v| v.as_text())
                .collect();
            let locality: Vec<&str> = doc
                .get_all(self.field_locality())
                .filter_map(|v| v.as_text())
                .collect();
            let s2cell = doc
                .get_first(self.field_s2cell())
                .unwrap()
                .as_u64()
                .unwrap();
            let cellid = s2::cellid::CellID(s2cell);
            let latlng = s2::latlng::LatLng::from(cellid);

            println!(
                "house_num: {:?}, road: {:?}, unit: {:?}, locality: {:?}, latlng: {:?}",
                house_num, road, unit, locality, latlng
            );
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
        doc.add_u64(self.schema.get_field(FIELD_S2CELL).unwrap(), poi.s2cell);
        self.tantivy_writer.add_document(doc)?;

        Ok(())
    }

    pub fn commit(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.tantivy_writer.commit()?;
        Ok(())
    }
}
