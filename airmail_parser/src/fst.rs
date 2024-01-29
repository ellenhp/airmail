use airmail_common::{
    dicts::KeyedFst,
    fst::{search_fst, FstMatchMode},
};
use nom::IResult;

use crate::common::{query_sep, query_term};

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
