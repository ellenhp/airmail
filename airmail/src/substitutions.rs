use std::{collections::HashMap, error::Error};

use lingua::{Language, LanguageDetector, LanguageDetectorBuilder};
use regex::Regex;

lazy_static! {
    static ref ASCII_WHITESPACE_RE: Regex = Regex::new(r"[ \t\r\n]+").unwrap();
}

lazy_static! {
    static ref EN_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/en/street_types.txt")).unwrap();
    static ref CA_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/ca/street_types.txt")).unwrap();
    static ref ES_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/es/street_types.txt")).unwrap();
    static ref AR_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/ar/street_types.txt")).unwrap();
    static ref FR_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/fr/street_types.txt")).unwrap();
    static ref DE_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/de/street_types.txt")).unwrap();
    static ref IT_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/it/street_types.txt")).unwrap();
    static ref PT_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/pt/street_types.txt")).unwrap();
    static ref RU_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/ru/street_types.txt")).unwrap();
    static ref ZH_STREET_TYPES: SubstitutionDict =
        SubstitutionDict::from_str(include_str!("../dictionaries/zh/street_types.txt")).unwrap();
    static ref LANGUAGE_CLASSIFIER: LanguageDetector =
        LanguageDetectorBuilder::from_all_languages().build();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub(super) struct SubstitutionDict {
    subs: Vec<(String, Vec<String>)>,
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
        Ok(Self {
            subs: subs.into_iter().collect(),
        })
    }

    pub fn substitute(&self, token: &str) -> Vec<String> {
        let mut substitutions = vec![token.to_string()];
        for (key, subs) in &self.subs {
            if key == token {
                substitutions.extend(subs.clone());
            }
        }
        substitutions
    }
}

fn sanitize(field: &str) -> String {
    ASCII_WHITESPACE_RE
        .replace_all(&deunicode::deunicode(field).to_lowercase(), " ")
        .to_string()
}

pub(super) fn apply_subs(
    prefix: &[String],
    remaining: &[String],
    dict: &SubstitutionDict,
) -> Result<Vec<String>, Box<dyn Error>> {
    if remaining.is_empty() {
        return Ok(vec![prefix.join(" ")]);
    }

    let mut permutations = vec![];

    for sub in dict.substitute(&remaining[0]) {
        let mut prefix = prefix.to_vec();
        prefix.push(sub);
        let mut remaining = remaining.to_vec();
        remaining.remove(0);
        permutations.extend(apply_subs(&prefix, &remaining, dict)?);
    }

    Ok(permutations)
}

pub fn permute_road(road: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let sub_dict: &SubstitutionDict = match LANGUAGE_CLASSIFIER.detect_language_of(road) {
        Some(Language::English) => &EN_STREET_TYPES,
        Some(Language::Arabic) => &AR_STREET_TYPES,
        Some(Language::Spanish) => &ES_STREET_TYPES,
        Some(Language::French) => &FR_STREET_TYPES,
        Some(Language::German) => &DE_STREET_TYPES,
        Some(Language::Italian) => &IT_STREET_TYPES,
        Some(Language::Portuguese) => &PT_STREET_TYPES,
        Some(Language::Russian) => &RU_STREET_TYPES,
        Some(Language::Chinese) => &ZH_STREET_TYPES,
        Some(Language::Catalan) => &CA_STREET_TYPES,
        _ => return Ok(vec![sanitize(road)]),
    };
    let road_tokens: Vec<String> = sanitize(road)
        .split_ascii_whitespace()
        .map(|s| s.to_string())
        .collect();
    apply_subs(&vec![], &road_tokens, sub_dict)
}

#[cfg(test)]
mod test {
    use crate::substitutions::permute_road;

    #[test]
    fn test_permute_road() {
        let road = "fremont ave n";
        let permutations = permute_road(road).unwrap();
        dbg!(permutations.clone());
        assert_eq!(permutations.len(), 3);
    }

    #[test]
    fn test_permute_road_cat() {
        let road = "carrer de villarroel";
        let permutations = permute_road(road).unwrap();
        dbg!(permutations.clone());
        assert_eq!(permutations.len(), 3);
    }
}
