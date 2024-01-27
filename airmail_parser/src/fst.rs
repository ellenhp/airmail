use std::{
    hash::Hash,
    sync::{Arc, Mutex},
};

use cached::proc_macro::cached;
use fst::{
    automaton::{Levenshtein, Str},
    Automaton, IntoStreamer, Streamer,
};
use nom::IResult;

use crate::common::{query_sep, query_term};

// Hold the global key count in a mutex.
lazy_static! {
    static ref KEY_COUNT: Mutex<usize> = Mutex::new(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FstKey(usize);

#[derive(Debug, Clone)]
pub struct KeyedFst {
    fst: Arc<fst::Set<Vec<u8>>>,
    key: FstKey,
}

impl Hash for KeyedFst {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl PartialEq for KeyedFst {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for KeyedFst {}

impl KeyedFst {
    pub fn new(fst: fst::Set<Vec<u8>>) -> Self {
        let mut key_count = KEY_COUNT.lock().unwrap();
        let key = FstKey(*key_count);
        *key_count += 1;
        Self {
            fst: Arc::new(fst),
            key,
        }
    }

    pub fn key(&self) -> FstKey {
        self.key
    }

    pub fn fst(&self) -> &fst::Set<Vec<u8>> {
        &self.fst
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FstMatchMode {
    Prefix,
    Levenshtein(u32),
    GreedyLevenshtein(u32),
}

#[cached(size = 131072)]
fn search_fst(fst: KeyedFst, query: String, dist: u32, prefix: bool) -> bool {
    if dist > 0 {
        if prefix {
            fst.fst
                .search(Levenshtein::new(&query, dist).unwrap().starts_with())
                .into_stream()
                .next()
                .is_some()
        } else {
            fst.fst
                .search(Levenshtein::new(&query, dist).unwrap())
                .into_stream()
                .next()
                .is_some()
        }
    } else {
        if prefix {
            fst.fst
                .search(Str::new(&query).starts_with())
                .into_stream()
                .next()
                .is_some()
        } else {
            fst.fst
                .search(Str::new(&query))
                .into_stream()
                .next()
                .is_some()
        }
    }
}

pub fn parse_fst<'a>(
    fst: &KeyedFst,
    match_mode: FstMatchMode,
    input: &'a str,
) -> IResult<&'a str, &'a str> {
    match match_mode {
        FstMatchMode::Prefix => {
            let (remainder, term) = query_term(input)?;
            if search_fst(fst.clone(), term.to_string(), 0, true) {
                Ok((remainder, term))
            } else {
                Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Fail,
                )))
            }
        }
        FstMatchMode::Levenshtein(dist) => {
            let (remainder, term) = query_term(input)?;
            if search_fst(fst.clone(), input.to_string(), dist, false) {
                Ok((remainder, term))
            } else {
                Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Fail,
                )))
            }
        }
        FstMatchMode::GreedyLevenshtein(dist) => {
            let mut matching_slice_length = 0usize;
            let mut sep_length = 0usize;
            loop {
                let remaining_input = &input[matching_slice_length + sep_length..input.len()];
                if remaining_input.is_empty() {
                    break;
                }
                let (remainder, term) = if let Ok((remainder, term)) = query_term(remaining_input) {
                    (remainder, term)
                } else {
                    break;
                };
                let tentative_slice = &input[0..matching_slice_length + sep_length + term.len()];
                let have_match = search_fst(fst.clone(), tentative_slice.to_string(), dist, true);
                if have_match {
                    matching_slice_length += sep_length + term.len();
                    if let Ok((_, matched_sep)) = query_sep(remainder) {
                        sep_length = matched_sep.len();
                    } else {
                        sep_length = 0;
                    }
                } else {
                    break;
                }
            }
            if matching_slice_length == 0 {
                Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Fail,
                )))
            } else {
                // Double-check that the slice we found is actually a match, and not just a prefix of a match.
                let tentative_slice = &input[0..matching_slice_length];
                let have_match = search_fst(fst.clone(), tentative_slice.to_string(), dist, false);
                if have_match {
                    Ok((
                        &input[matching_slice_length..input.len()],
                        &input[0..matching_slice_length],
                    ))
                } else {
                    Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Fail,
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{parse_fst, FstMatchMode, KeyedFst};

    const MAIN_STREET_STRS: &[&str] = &["main street", "main st", "main", "grocery"];

    fn fst_from_strs(strs: &[&str]) -> KeyedFst {
        let mut builder = fst::SetBuilder::memory();
        let mut strs: Vec<_> = strs.iter().map(|s| s.to_lowercase()).collect();
        strs.sort();
        for s in strs {
            builder.insert(s).unwrap();
        }
        let fst = builder.into_set();
        KeyedFst::new(fst)
    }

    #[test]
    fn test_greedy() {
        // The greedy levenshtein match mode should match the longest possible substring in a given query.
        let set = fst_from_strs(MAIN_STREET_STRS);
        {
            let (remainder, matched) =
                parse_fst(&set, FstMatchMode::GreedyLevenshtein(0), "main street city").unwrap();
            assert_eq!(matched, "main street");
            assert_eq!(remainder, " city");
        }
        {
            let (remainder, matched) =
                parse_fst(&set, FstMatchMode::GreedyLevenshtein(0), "main st city").unwrap();
            assert_eq!(matched, "main st");
            assert_eq!(remainder, " city");
        }
        {
            let (remainder, matched) =
                parse_fst(&set, FstMatchMode::GreedyLevenshtein(0), "main city").unwrap();
            assert_eq!(matched, "main");
            assert_eq!(remainder, " city");
        }
    }

    #[test]
    fn test_nongreedy() {
        // Regardless of what the query is we should always match the first term.
        let set = fst_from_strs(MAIN_STREET_STRS);
        {
            let (remainder, matched) =
                parse_fst(&set, FstMatchMode::Levenshtein(0), "main street").unwrap();
            assert_eq!(matched, "main");
            assert_eq!(remainder, " street");
        }
        {
            let (remainder, matched) =
                parse_fst(&set, FstMatchMode::Levenshtein(0), "main st").unwrap();
            assert_eq!(matched, "main");
            assert_eq!(remainder, " st");
        }
        {
            let (remainder, matched) =
                parse_fst(&set, FstMatchMode::Levenshtein(0), "main").unwrap();
            assert_eq!(matched, "main");
            assert_eq!(remainder, "");
        }
    }

    #[test]
    fn test_prefix() {
        let set = fst_from_strs(MAIN_STREET_STRS);
        let (remainder, matched) = parse_fst(&set, FstMatchMode::Prefix, "mai").unwrap();
        assert_eq!(matched, "mai");
        assert_eq!(remainder, "");
    }
}

// struct FstParser<'a> {
//     fst: fst::Set<&'a [u8]>,
// }

// impl<'a> FstParser<'a> {
//     pub fn new(data: &'a [u8]) -> Result<Self, Box<dyn Error>> {
//         let fst = fst::Set::new(data)?;
//         Ok(Self { fst })
//     }
// }

// impl<'a, O, E> Parser<&str, O, E> for FstParser<'a> {
//     fn parse(&mut self, input: &str) -> nom::IResult<&str, O, E> {
//         let (input, bytes) = take_while(|c| c.is_ascii_alphanumeric())(input)?;
//         let bytes = bytes.as_bytes();
//         let bytes = self.fst.find(bytes);
//         Ok((input, bytes))
//     }
// }
