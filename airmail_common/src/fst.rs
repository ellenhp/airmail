use std::hash::Hash;

use crate::dicts::KeyedFst;
use cached::proc_macro::cached;
use fst::{
    automaton::{Levenshtein, Str},
    Automaton, IntoStreamer, Streamer,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FstMatchMode {
    Prefix,
    Levenshtein(u32),
    GreedyLevenshtein(u32),
}

#[cached(size = 131072)]
pub fn search_fst(fst: KeyedFst, query: String, dist: u32, prefix: bool) -> bool {
    if dist > 0 {
        if prefix {
            fst.fst()
                .search(Levenshtein::new(&query, dist).unwrap().starts_with())
                .into_stream()
                .next()
                .is_some()
        } else {
            fst.fst()
                .search(Levenshtein::new(&query, dist).unwrap())
                .into_stream()
                .next()
                .is_some()
        }
    } else {
        if prefix {
            fst.fst()
                .search(Str::new(&query).starts_with())
                .into_stream()
                .next()
                .is_some()
        } else {
            fst.fst()
                .search(Str::new(&query))
                .into_stream()
                .next()
                .is_some()
        }
    }
}
