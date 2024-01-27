use levenshtein_automata::{Distance, LevenshteinAutomatonBuilder};
use tantivy::{
    query::{BitSetDocSet, ConstScorer, Query, Weight},
    schema::IndexRecordOption,
};
use tantivy_common::BitSet;

#[derive(Debug, Clone)]
pub enum AddressComponent {
    House,
    Category,
    HouseNumber,
    Road,
    Unit,
    City,
    State,
    Country,
}

#[derive(Debug, Clone)]
pub struct AirmailQueryComponent {
    component: AddressComponent,
    value: String,
    match_prefix: bool,
}

impl AirmailQueryComponent {
    pub fn new(component: AddressComponent, value: String, match_prefix: bool) -> Self {
        Self {
            component,
            value,
            match_prefix,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AirmailQuery {
    field: tantivy::schema::Field,
    components: Vec<AirmailQueryComponent>,
}

impl AirmailQuery {
    pub fn new(field: tantivy::schema::Field, components: Vec<AirmailQueryComponent>) -> Self {
        Self { field, components }
    }
}

impl Query for AirmailQuery {
    fn weight(
        &self,
        _enable_scoring: tantivy::query::EnableScoring<'_>,
    ) -> tantivy::Result<Box<dyn tantivy::query::Weight>> {
        Ok(Box::new(AirmailQueryWeight {
            field: self.field,
            components: self.components.clone(),
        }) as Box<dyn tantivy::query::Weight>)
    }
}

pub struct AirmailQueryWeight {
    field: tantivy::schema::Field,
    components: Vec<AirmailQueryComponent>,
}

impl Weight for AirmailQueryWeight {
    fn scorer(
        &self,
        reader: &tantivy::SegmentReader,
        boost: tantivy::Score,
    ) -> tantivy::Result<Box<dyn tantivy::query::Scorer>> {
        let max_doc = reader.max_doc();
        let inverted_index = reader.inverted_index(self.field)?;
        let term_dict = inverted_index.terms();

        let mut doc_bitsets = Vec::new();

        for component in &self.components {
            let mut doc_bitset = BitSet::with_max_value(max_doc);
            // match component.component {
            //     AddressComponent::HouseNumber => todo!(),
            //     AddressComponent::Road => todo!(),
            //     AddressComponent::City => todo!(),
            //     _ => todo!(),
            // }
            let max_distance = match component.component {
                AddressComponent::HouseNumber => 0,
                _ => usize::min(2, component.value.len() / 3) as u8,
            };
            let builder = LevenshteinAutomatonBuilder::new(max_distance, true);
            let automaton = if component.match_prefix && component.value.len() > 2 {
                builder.build_prefix_dfa(&component.value)
            } else {
                builder.build_dfa(&component.value)
            };
            let mut streamer = term_dict.stream()?;
            while streamer.advance() {
                let term = streamer.key();
                if match automaton.eval(term) {
                    Distance::Exact(dist) => dbg!(dist) <= 2,
                    Distance::AtLeast(_dist) => false,
                } {
                    println!("match: {}", std::str::from_utf8(term).unwrap());
                    let mut block_segment_postings = inverted_index
                        .read_block_postings_from_terminfo(
                            streamer.value(),
                            IndexRecordOption::Basic,
                        )?;
                    loop {
                        let docs = block_segment_postings.docs();
                        if docs.is_empty() {
                            break;
                        }
                        for &doc in docs {
                            doc_bitset.insert(doc);
                        }
                        block_segment_postings.advance();
                    }
                }
            }
            doc_bitsets.push(doc_bitset);
        }
        let mut final_bitset = doc_bitsets.pop().unwrap();
        for doc_bitset in doc_bitsets {
            final_bitset.intersect_update(&((&doc_bitset).into()));
        }
        Ok(Box::new(ConstScorer::new(
            BitSetDocSet::from(final_bitset),
            boost,
        )))
    }

    fn explain(
        &self,
        _reader: &tantivy::SegmentReader,
        _doc: tantivy::DocId,
    ) -> tantivy::Result<tantivy::query::Explanation> {
        todo!()
    }
}
