use std::collections::HashMap;

use crate::similarity::{Score, tf::Tf, token::Token};

#[derive(Default)]
pub struct DfScore(Score);

pub struct Df<'a> {
    map: HashMap<Token<'a>, DfScore>,
    num_docs: usize,
}
impl<'a> From<&[Tf<'a>]> for Df<'a> {
    fn from(value: &[Tf<'a>]) -> Self {
        let num_docs = value.len();
        let map = value
            .iter()
            .fold(HashMap::new(), |mut acc: HashMap<Token, DfScore>, curr| {
                let curr_map = curr.borrow_map();
                curr_map
                    .keys()
                    .for_each(|k| acc.entry(*k).or_default().0 += 1 as Score);
                acc
            });
        Self { num_docs, map }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100_000))]
        #[test]
        fn num_docs_matches_len(
            docs: Vec<Vec<Token<'_>>>
        ) {
            let tfs: Vec<Tf> = docs
                .iter()
                .map(|doc_tokens| Tf::from(doc_tokens.iter().copied()))
                .collect();

            let df = Df::from(tfs.as_slice());

            // `num_docs` is correct
            prop_assert_eq!(
                df.num_docs,
                tfs.len(),
                "The `num_docs` field should equal the number of input documents"
            );
        }
        #[test]
        fn unique_tokens_conservation(
            docs: Vec<Vec<Token<'_>>>
        ) {
            let tfs: Vec<Tf> = docs
                .iter()
                .map(|doc_tokens| Tf::from(doc_tokens.iter().copied()))
                .collect();

            let df = Df::from(tfs.as_slice());

            // Find all unique tokens that should exist in the final Df map.
            let all_unique_tokens: HashSet<Token> = tfs
                .iter()
                .flat_map(|tf| tf.borrow_map().keys().copied())
                .collect();

            prop_assert_eq!(
                df.map.len(),
                all_unique_tokens.len(),
                "The number of keys in the Df map should equal the total number of unique tokens"
            );
            // Iterate over every token that we know should be in the map.
            for &token in &all_unique_tokens {
                // Get the calculated DF score from the map.
                let df_score = df.map.get(&token)
                    .expect("A unique token from the input was missing from the Df map");

                // Check Property 2 (Score Range)
                prop_assert!(df_score.0 > 0 as Score, "DF score must be positive");
                prop_assert!(
                    df_score.0 as usize <= df.num_docs,
                    "DF score cannot be greater than the number of documents"
                );

                // Check Property 3 (Score Correctness)
                // Recalculate the expected score manually.
                let expected_df_count = tfs
                    .iter()
                    .filter(|tf| tf.borrow_map().contains_key(&token))
                    .count();

                prop_assert_eq!(
                    df_score.0,
                    expected_df_count as Score,
                    "The DF score for token '{:?}' is incorrect", token
                );
            }
        }
        #[test]
        fn score_is_positive(
            docs: Vec<Vec<Token<'_>>>
        ) {
            let tfs: Vec<Tf> = docs
                .iter()
                .map(|doc_tokens| Tf::from(doc_tokens.iter().copied()))
                .collect();

            let df = Df::from(tfs.as_slice());

            // Find all unique tokens that should exist in the final Df map.
            let all_unique_tokens: HashSet<Token> = tfs
                .iter()
                .flat_map(|tf| tf.borrow_map().keys().copied())
                .collect();
            // Iterate over every token that we know should be in the map.
            for &token in &all_unique_tokens {
                let df_score = df.map.get(&token)
                    .expect("A unique token from the input was missing from the Df map");

                // Check Property 2 (Score Range)
                prop_assert!(df_score.0 > 0 as Score, "DF score must be positive");
            }
        }
        #[test]
        fn score_is_no_larger_than_num_docs(
            docs: Vec<Vec<Token<'_>>>
        ) {
            let tfs: Vec<Tf> = docs
                .iter()
                .map(|doc_tokens| Tf::from(doc_tokens.iter().copied()))
                .collect();

            let df = Df::from(tfs.as_slice());

            // Find all unique tokens that should exist in the final Df map.
            let all_unique_tokens: HashSet<Token> = tfs
                .iter()
                .flat_map(|tf| tf.borrow_map().keys().copied())
                .collect();
            // Iterate over every token that we know should be in the map.
            for &token in &all_unique_tokens {
                let df_score = df.map.get(&token)
                    .expect("A unique token from the input was missing from the Df map");

                // Check Property 2 (Score Range)
                prop_assert!(
                    df_score.0 as usize <= df.num_docs,
                    "DF score cannot be greater than the number of documents"
                );

                // Check Property 3 (Score Correctness)
                // Recalculate the expected score manually.
                let expected_df_count = tfs
                    .iter()
                    .filter(|tf| tf.borrow_map().contains_key(&token))
                    .count();

                prop_assert_eq!(
                    df_score.0,
                    expected_df_count as Score,
                    "The DF score for token '{:?}' is incorrect", token
                );
            }
        }
        #[test]
        fn score_is_correct(
            docs: Vec<Vec<Token<'_>>>
        ) {
            let tfs: Vec<Tf> = docs
                .iter()
                .map(|doc_tokens| Tf::from(doc_tokens.iter().copied()))
                .collect();

            let df = Df::from(tfs.as_slice());

            // Find all unique tokens that should exist in the final Df map.
            let all_unique_tokens: HashSet<Token> = tfs
                .iter()
                .flat_map(|tf| tf.borrow_map().keys().copied())
                .collect();
            // Iterate over every token that we know should be in the map.
            for &token in &all_unique_tokens {
                let df_score = df.map.get(&token)
                    .expect("a unique token from the input was missing from the df map");

                // Check Property 3 (Score Correctness)
                // Recalculate the expected score manually.
                let expected_df_count = tfs
                    .iter()
                    .filter(|tf| tf.borrow_map().contains_key(&token))
                    .count();

                prop_assert_eq!(
                    df_score.0,
                    expected_df_count as Score,
                    "The DF score for token '{:?}' is incorrect", token
                );
            }
        }
    }
}
