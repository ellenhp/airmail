use std::collections::{HashMap, HashSet};

use cached::proc_macro::cached;
use itertools::Itertools;
use lingua::{LanguageDetector, LanguageDetectorBuilder};
use tantivy::tokenizer::{
    self, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer, TextAnalyzer, TokenStream,
};
use tantivy_jieba::JiebaTokenizer;

pub(crate) fn all_subsequences(tokens: &[String]) -> Vec<Vec<String>> {
    let mut subsequences: Vec<Vec<String>> = Vec::new();
    for i in 0..tokens.len() {
        for j in i..tokens.len() {
            subsequences.push(tokens[i..=j].iter().map(|s| s.to_string()).collect_vec());
        }
    }
    subsequences
}

thread_local! {
    static EXPANDER: QueryExpander = QueryExpander::new();
}

pub(crate) struct QueryExpander {
    pub(crate) detector: LanguageDetector,
    pub(crate) tokenizer: TextAnalyzer,
    pub(crate) stemmers: HashMap<lingua::Language, TextAnalyzer>,
    pub(crate) special_cases:
        HashMap<lingua::Language, Vec<Box<dyn Send + Fn(&str) -> Vec<String>>>>,
}

impl QueryExpander {
    pub(crate) fn new() -> QueryExpander {
        let detector = LanguageDetectorBuilder::from_all_languages().build();
        let language_map: HashMap<lingua::Language, tokenizer::Language> = [
            (lingua::Language::Arabic, tokenizer::Language::Arabic),
            (lingua::Language::Danish, tokenizer::Language::Danish),
            (lingua::Language::Dutch, tokenizer::Language::Dutch),
            (lingua::Language::English, tokenizer::Language::English),
            (lingua::Language::Finnish, tokenizer::Language::Finnish),
            (lingua::Language::French, tokenizer::Language::French),
            (lingua::Language::German, tokenizer::Language::German),
            (lingua::Language::Greek, tokenizer::Language::Greek),
            (lingua::Language::Hungarian, tokenizer::Language::Hungarian),
            (lingua::Language::Italian, tokenizer::Language::Italian),
            (
                lingua::Language::Portuguese,
                tokenizer::Language::Portuguese,
            ),
            (lingua::Language::Romanian, tokenizer::Language::Romanian),
            (lingua::Language::Russian, tokenizer::Language::Russian),
            (lingua::Language::Spanish, tokenizer::Language::Spanish),
            (lingua::Language::Swedish, tokenizer::Language::Swedish),
            (lingua::Language::Tamil, tokenizer::Language::Tamil),
            (lingua::Language::Turkish, tokenizer::Language::Turkish),
        ]
        .into_iter()
        .collect();
        let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .build();
        let mut stemmers: HashMap<lingua::Language, TextAnalyzer> = language_map
            .iter()
            .map(|(lingua_language, tantivy_language)| {
                (
                    *lingua_language,
                    TextAnalyzer::builder(SimpleTokenizer::default())
                        .filter(RemoveLongFilter::limit(40))
                        .filter(LowerCaser)
                        .filter(Stemmer::new(*tantivy_language))
                        .build(),
                )
            })
            .collect();
        stemmers.insert(
            lingua::Language::Chinese,
            TextAnalyzer::builder(JiebaTokenizer {})
                .filter(RemoveLongFilter::limit(40))
                .filter(LowerCaser)
                .build(),
        );
        let mut special_cases: HashMap<
            lingua::Language,
            Vec<Box<dyn Send + Fn(&str) -> Vec<String>>>,
        > = HashMap::new();
        special_cases.insert(
            lingua::Language::English,
            vec![Box::new(|text: &str| {
                let mut texts = vec![];
                if text.contains("'s") {
                    texts.push(text.replace("'s", ""));
                }
                texts
            })],
        );
        QueryExpander {
            detector,
            tokenizer,
            stemmers,
            special_cases,
        }
    }
}

pub(crate) fn lang_stem(text: &str, language: &lingua::Language) -> Vec<String> {
    let analyzer = EXPANDER.with(|expander| expander.stemmers.get(language).cloned());
    if let Some(mut analyzer) = analyzer {
        let mut token_stream = analyzer.token_stream(&text);
        let mut tokens = Vec::new();
        while let Some(token) = token_stream.next() {
            tokens.push(deunicode::deunicode(&token.text).to_lowercase());
        }
        return tokens;
    }
    deunicode::deunicode(text)
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

#[cached]
pub(crate) fn all_possible_queries(text: String) -> Vec<String> {
    let mut queries = HashSet::new();

    if let Some(lang) = EXPANDER.with(|expander| {
        let lang = expander.detector.detect_language_of(&text);
        if let Some(lang) = lang {
            if expander.detector.compute_language_confidence(&text, lang) > 0.5 {
                Some(lang)
            } else {
                None
            }
        } else {
            None
        }
    }) {
        EXPANDER.with(|expander| {
            for special_case in expander.special_cases.get(&lang).unwrap_or(&vec![]) {
                // These special cases do things like (for English) handling apostrophes. They may emit multiple reprocessed texts.
                let texts = special_case(&text);
                for text in texts {
                    let lang_tokens = lang_stem(&text, &lang);
                    for subsequence in all_subsequences(&lang_tokens) {
                        queries.insert(subsequence);
                    }
                }
            }
        });
        let lang_tokens = lang_stem(&text, &lang);
        for subsequence in all_subsequences(&lang_tokens) {
            queries.insert(subsequence);
        }
    } else {
        let mut analyzer = EXPANDER.with(|expander| expander.tokenizer.clone());
        let mut token_stream = analyzer.token_stream(&text);
        let mut tokens = Vec::new();
        while let Some(token) = token_stream.next() {
            tokens.push(deunicode::deunicode(&token.text).to_lowercase());
        }
        for subsequence in all_subsequences(&tokens) {
            queries.insert(subsequence);
        }
    }

    queries
        .into_iter()
        .map(|q| q.into_iter().filter(|s| s.len() > 1).collect())
        .filter(|q: &Vec<String>| !q.is_empty())
        .map(|q| q.join(" "))
        .collect()
}

#[cfg(test)]
mod test {
    use lingua::Language;

    use crate::query::{all_possible_queries, lang_stem};

    #[test]
    fn test_expand_ambiguous() {
        let queries = super::all_possible_queries("QFC".to_string());
        dbg!(&queries);
        assert_eq!(queries.len(), 1);
        assert!(queries.contains(&"qfc".to_string()));
    }

    #[test]
    fn test_expand_en() {
        let queries = all_possible_queries("dick's wallingford".to_string());
        dbg!(&queries);
        assert_eq!(queries.len(), 3);
        assert!(queries.contains(&"dick".to_string()));
        assert!(queries.contains(&"dick wallingford".to_string()));
        assert!(queries.contains(&"wallingford".to_string()));
    }

    #[test]
    fn test_lang_stem_zh() {
        let queries = lang_stem("太空针塔", &Language::Chinese);
        dbg!(&queries);
        assert_eq!(queries.len(), 2);
        assert!(queries.contains(&"tai kong".to_string()));
        assert!(queries.contains(&"zhen ta".to_string()));
    }
}
