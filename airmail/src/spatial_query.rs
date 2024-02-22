use itertools::Itertools;
use s2::cellid::MAX_LEVEL;
use tantivy::{
    columnar::Column,
    query::{EnableScoring, Query, ScoreCombiner, Scorer, Weight},
    DocSet, Score, SegmentReader, Term, TERMINATED,
};

#[derive(Debug)]
pub struct SpatialQuery {
    inner: Box<dyn Query>,
    s2cell_parents: Vec<u64>,
    field: String,
}

impl Clone for SpatialQuery {
    fn clone(&self) -> Self {
        SpatialQuery {
            inner: self.inner.box_clone(),
            s2cell_parents: self.s2cell_parents.clone(),
            field: self.field.clone(),
        }
    }
}

impl Query for SpatialQuery {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        Ok(Box::new(SpatialWeight {
            inner: self.inner.weight(enable_scoring)?,
            values: self.s2cell_parents.clone(),
            field: self.field.clone(),
        }))
    }

    fn query_terms<'a>(&'a self, visitor: &mut dyn FnMut(&'a Term, bool)) {
        self.inner.query_terms(visitor);
    }
}

impl SpatialQuery {
    pub fn new(inner: Box<dyn Query>, s2cells: Vec<u64>, field: String) -> SpatialQuery {
        SpatialQuery {
            inner,
            s2cell_parents: s2cells,
            field,
        }
    }
}

/// Just ignores scores. The `DoNothingCombiner` does not
/// even call the scorers `.score()` function.
///
/// It is useful to optimize the case when scoring is disabled.
#[derive(Default, Clone, Copy)] //< these should not be too much work :)
pub struct DoNothingCombiner;

impl ScoreCombiner for DoNothingCombiner {
    fn update<TScorer: Scorer>(&mut self, _scorer: &mut TScorer) {}

    fn clear(&mut self) {}

    fn score(&self) -> Score {
        1.0
    }
}

pub struct SpatialScorer {
    reader: Column,
    value_mask_pairs: Vec<(u64, u64)>,
    first: bool,
    inner: Box<dyn Scorer>,
}

impl SpatialScorer {
    pub fn new(
        reader: &SegmentReader,
        values: Vec<u64>,
        field: &str,
        inner: Box<dyn Scorer>,
    ) -> SpatialScorer {
        let reader = reader.fast_fields().u64(field).unwrap();
        let value_mask_pairs = values
            .iter()
            .map(|v| {
                let cell = s2::cellid::CellID(*v);
                let lsb_mask = 1u64 << (2 + 2 * (MAX_LEVEL - cell.level()));
                let msb_mask = !(lsb_mask - 1);
                (v & msb_mask, msb_mask)
            })
            .collect_vec();

        let mut scorer = SpatialScorer {
            reader,
            value_mask_pairs,
            first: false,
            inner,
        };
        scorer.advance();
        scorer.first = true;
        scorer
    }
}

impl DocSet for SpatialScorer {
    fn advance(&mut self) -> tantivy::DocId {
        if self.first {
            self.first = false;
            return self.inner.doc();
        }
        loop {
            let doc = self.inner.advance();
            if doc == TERMINATED {
                return TERMINATED;
            }

            let s2cell = self.reader.first(doc).unwrap();
            for (value, mask) in &self.value_mask_pairs {
                if s2cell & mask == *value {
                    return doc;
                }
            }
        }
    }

    fn doc(&self) -> tantivy::DocId {
        self.inner.doc()
    }

    fn size_hint(&self) -> u32 {
        self.inner.size_hint()
    }

    fn seek(&mut self, target: tantivy::DocId) -> tantivy::DocId {
        self.inner.seek(target)
    }
}

impl Scorer for SpatialScorer {
    fn score(&mut self) -> Score {
        1.0
    }
}

/// Weight associated to the `BoolQuery`.
pub struct SpatialWeight {
    values: Vec<u64>,
    field: String,
    inner: Box<dyn Weight>,
}

impl Weight for SpatialWeight {
    fn scorer(
        &self,
        reader: &tantivy::SegmentReader,
        boost: Score,
    ) -> tantivy::Result<Box<dyn Scorer>> {
        Ok(Box::new(SpatialScorer::new(
            reader,
            self.values.clone(),
            &self.field,
            self.inner.scorer(reader, boost)?,
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
