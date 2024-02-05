use std::{collections::HashMap, error::Error};

use airmail_common::{dicts::street_suffixes_fst, fst::search_fst};
use regex::Regex;

lazy_static! {
    static ref ASCII_WHITESPACE_RE: Regex = Regex::new(r"[ \t\r\n]+").unwrap();
    static ref STREET_SUFFIXES_SUBS: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../permute_dicts/en/street_types.txt")).unwrap();
}

pub(super) struct SubstitutionDict {
    subs: HashMap<String, Vec<String>>,
}

impl SubstitutionDict {
    pub(super) fn from_str(contents: &str) -> Result<Self, Box<dyn Error>> {
        let mut subs: HashMap<String, Vec<String>> = HashMap::new();
        for line in contents.lines() {
            let components: Vec<_> = line.split('|').collect();
            for component in &components {
                if let Some(existing_subs) = subs.get_mut(*component) {
                    for component_to_add in &components {
                        if !existing_subs.contains(&component_to_add.to_string()) {
                            existing_subs.push(component_to_add.to_string());
                        }
                    }
                } else {
                    subs.insert(
                        component.to_string(),
                        components.iter().map(|s| s.to_string()).collect(),
                    );
                }
            }
        }
        Ok(Self { subs })
    }

    pub fn substitute(&self, token: &str) -> Vec<String> {
        let mut substitutions = vec![token.to_string()];
        if let Some(subs) = self.subs.get(token) {
            substitutions.extend(subs.clone());
        }
        substitutions
    }
}

fn sanitize(field: &str) -> String {
    ASCII_WHITESPACE_RE
        .replace_all(&deunicode::deunicode(field).to_lowercase(), " ")
        .to_string()
}

fn permute(prefix: &str, candidates: &[Vec<String>]) -> Vec<String> {
    let candidates_this_round = if let Some(first) = candidates.first() {
        first
    } else {
        return vec![prefix.trim().to_string()];
    };
    let mut permutations = Vec::new();
    for candidate in candidates_this_round {
        let mut base = prefix.to_string();
        base.push_str(candidate);
        base.push(' ');
        permutations.extend(permute(&base, &candidates[1..]));
    }
    permutations
}

pub(super) fn permute_housenum(housenum: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut permutations = Vec::new();
    permutations.push(sanitize(housenum));
    Ok(permutations)
}

pub(super) fn permute_road(road: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut permutations = Vec::new();
    let road = sanitize(road);
    // This may be a bad way of handling it, I don't know enough about non-ascii whitespace to be sure.
    let road_components: Vec<Vec<String>> = road
        .split_whitespace()
        .map(|s| STREET_SUFFIXES_SUBS.substitute(s))
        .collect();
    let mut found_suffix = false;
    for i in 0..=road_components.len() {
        let base_substrings = permute("", &road_components[0..i]);
        let suffix_substrings = permute("", &road_components[i..]);
        if !found_suffix {
            for substring_pair in base_substrings.iter().zip(suffix_substrings.iter()) {
                let suffix_substring = substring_pair.1.clone();
                if search_fst(street_suffixes_fst(), suffix_substring.clone(), 0, false) {
                    found_suffix = true;
                }
            }
        }

        if found_suffix {
            permutations.extend(base_substrings.iter().cloned());
        }
    }
    Ok(permutations)
}

pub(super) fn permute_unit(unit: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut permutations = Vec::new();
    permutations.push(sanitize(unit));
    Ok(permutations)
}

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use super::permute_road;

    #[test]
    fn test_permute() {
        let candidates = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["c".to_string(), "d".to_string()],
        ];
        let permutations = super::permute("", &candidates);
        assert_eq!(permutations.len(), 4);
        assert!(permutations.contains(&"a c".to_string()));
        assert!(permutations.contains(&"a d".to_string()));
        assert!(permutations.contains(&"b c".to_string()));
        assert!(permutations.contains(&"b d".to_string()));
    }

    #[test]
    fn test_permute_road() {
        let road = "fremont ave n";
        let permutations: BTreeSet<String> = permute_road(road).unwrap().iter().cloned().collect();
        dbg!(permutations.clone());
        assert_eq!(permutations.len(), 3);
        assert!(permutations.contains("fremont ave n"));
        assert!(permutations.contains("fremont ave"));
        assert!(permutations.contains("fremont"));
    }
}
