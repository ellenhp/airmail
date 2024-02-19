use std::collections::HashSet;

use itertools::Itertools;

fn all_subsequences(tokens: &[&str]) -> Vec<Vec<String>> {
    let mut subsequences: Vec<Vec<String>> = Vec::new();
    for i in 0..tokens.len() {
        for j in i..tokens.len() {
            subsequences.push(tokens[i..=j].iter().map(|s| s.to_string()).collect_vec());
        }
    }
    subsequences
}

pub(crate) fn all_possible_queries(tokens: &[&str]) -> Vec<Vec<String>> {
    let mut queries = HashSet::new();
    for subsequence in all_subsequences(&tokens) {
        queries.insert(subsequence);
    }
    let processed_tokens: Vec<String> = tokens
        .iter()
        .map(|s| {
            if s.ends_with("'s") {
                s[..s.len() - 2].to_string()
            } else {
                s.to_string()
            }
        })
        .collect();
    for subsequence in all_subsequences(&processed_tokens.iter().map(|s| s.as_str()).collect_vec())
    {
        queries.insert(subsequence);
    }
    let processed_tokens: Vec<String> = tokens
        .iter()
        .map(|s| {
            if s.ends_with("'s") {
                format!("{}s", s[..s.len() - 2].to_string())
            } else {
                s.to_string()
            }
        })
        .collect();
    for subsequence in all_subsequences(&processed_tokens.iter().map(|s| s.as_str()).collect_vec())
    {
        queries.insert(subsequence);
    }
    queries.into_iter().collect()
}
