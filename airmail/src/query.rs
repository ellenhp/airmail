use itertools::Itertools;

pub(crate) fn all_subsequences(tokens: &[String]) -> Vec<Vec<String>> {
    let mut subsequences: Vec<Vec<String>> = Vec::new();
    for i in 0..tokens.len() {
        for j in i..tokens.len() {
            subsequences.push(tokens[i..=j].iter().map(|s| s.to_string()).collect_vec());
        }
    }
    subsequences
}
