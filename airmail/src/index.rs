use tantivy::{
    collector::{Count, TopDocs},
    schema::{Schema, INDEXED, STORED, TEXT},
};

use crate::{poi::AirmailPoi, query::AirmailQuery};

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

    pub fn field_region(&self) -> tantivy::schema::Field {
        self.tantivy_index.schema().get_field(FIELD_REGION).unwrap()
    }

    pub fn create(index_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let schema = Self::schema();
        let tantivy_index = tantivy::Index::create_in_dir(index_dir, schema)?;
        Ok(Self { tantivy_index })
    }

    pub fn new(index_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tantivy_index = tantivy::Index::open_in_dir(index_dir)?;
        Ok(Self { tantivy_index })
    }

    pub fn writer(&mut self) -> Result<AirmailIndexWriter, Box<dyn std::error::Error>> {
        let tantivy_writer = self.tantivy_index.writer(50_000_000)?;
        let writer = AirmailIndexWriter { tantivy_writer };
        Ok(writer)
    }

    pub fn search(&self, query: AirmailQuery) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let tantivy_reader = self.tantivy_index.reader()?;
        let searcher = tantivy_reader.searcher();
        let results = searcher.search(&query, &(TopDocs::with_limit(5), Count))?;
        let strings = results
            .0
            .iter()
            .map(|s| format!("{:?}", searcher.doc(s.1).unwrap()))
            .collect();
        Ok(strings)
    }
}

pub struct AirmailIndexWriter {
    tantivy_writer: tantivy::IndexWriter,
}

impl AirmailIndexWriter {
    pub fn add_poi(&mut self, poi: AirmailPoi) -> Result<(), Box<dyn std::error::Error>> {
        let mut document = tantivy::Document::new();
        let schema = self.tantivy_writer.index().schema();
        if let Some(name) = poi.name {
            document.add_text(schema.get_field(FIELD_NAME)?, name);
        }
        if let Some(category) = poi.category {
            document.add_text(schema.get_field(FIELD_CATEGORY)?, category);
        }
        document.add_u64(schema.get_field(FIELD_S2CELL)?, poi.s2cell);
        self.tantivy_writer.add_document(document)?;
        Ok(())
    }

    pub fn commit(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.tantivy_writer.commit()?;
        Ok(())
    }
}
