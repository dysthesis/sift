use std::collections::HashMap;

use crate::similarity::{Score, tf::Tf, token::Token};

#[derive(Default, Clone)]
pub struct DfScore(Score);

#[derive(Clone)]
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
pub struct Idf<'a>(HashMap<Token<'a>, Score>);
impl<'a> From<Df<'a>> for Idf<'a> {
    fn from(value: Df<'a>) -> Self {
        let res = value
            .map
            .into_iter()
            .map(|(term, df)| {
                let idf = ((value.num_docs as Score + 1 as Score) / (df.0 + 1 as Score)).ln() + 1.0;
                (term, idf)
            })
            .collect();
        Idf(res)
    }
}

impl<'a> Idf<'a> {
    pub fn get(&self, term: &Token<'a>) -> Option<Score> {
        self.0.get(term).copied()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;
    // A struct to hold the data needed by our tests.
    // This is cleaner than returning a big tuple.
    struct IdfTestData<'a> {
        idf: Idf<'a>,
        original_df: Df<'a>,
    }

    // A helper function to avoid repeating setup code in every test.
    fn setup_idf_test_data(docs_as_tokens: Vec<Vec<Token<'_>>>) -> IdfTestData<'_> {
        let tfs: Vec<Tf> = docs_as_tokens
            .iter()
            .map(|doc| Tf::from(doc.iter().copied()))
            .collect();

        let df = Df::from(tfs.as_slice());

        // Clone `df` because `Idf::from` consumes its input, but our tests
        // need the original data for assertions.
        let original_df = df.clone();
        let idf = Idf::from(df);

        IdfTestData { idf, original_df }
    }

    proptest! {
        #[test]
        fn num_docs_matches_len(
            docs: Vec<Vec<Token<'_>>>
        ) {
            let tfs: Vec<Tf> = docs
                .iter()
                .map(|doc_tokens| Tf::from(doc_tokens.iter().copied()))
                .collect();

            let df = Df::from(tfs.as_slice());

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

            let all_unique_tokens: HashSet<Token> = tfs
                .iter()
                .flat_map(|tf| tf.borrow_map().keys().copied())
                .collect();

            prop_assert_eq!(
                df.map.len(),
                all_unique_tokens.len(),
                "The number of keys in the Df map should equal the total number of unique tokens"
            );
            for &token in &all_unique_tokens {
                let df_score = df.map.get(&token)
                    .expect("A unique token from the input was missing from the Df map");

                prop_assert!(df_score.0 > 0 as Score, "DF score must be positive");
                prop_assert!(
                    df_score.0 as usize <= df.num_docs,
                    "DF score cannot be greater than the number of documents"
                );

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

        #[test]
        fn idf_map_preserves_all_tokens(docs_as_tokens: Vec<Vec<Token<'_>>>) {
            let test_data = setup_idf_test_data(docs_as_tokens);

            prop_assert_eq!(
                test_data.idf.0.len(),
                test_data.original_df.map.len(),
                "IDF map should have the same number of tokens as the DF map"
            );

            for token in test_data.original_df.map.keys() {
                prop_assert!(
                    test_data.idf.0.contains_key(token),
                    "Token {:?} from DF map was missing in IDF map", token
                );
            }
        }

        #[test]
        fn idf_score_is_always_positive_and_smoothed(docs_as_tokens: Vec<Vec<Token<'_>>>) {
            let test_data = setup_idf_test_data(docs_as_tokens);

            for (token, idf_score) in &test_data.idf.0 {
                // Your formula guarantees a score of at least 1.0
                prop_assert!(
                    *idf_score >= 1.0,
                    "IDF score for token {:?} was {}, but should be >= 1.0",
                    token, idf_score
                );
            }
        }

        #[test]
        fn idf_score_is_calculated_correctly(docs_as_tokens: Vec<Vec<Token<'_>>>) {
            let test_data = setup_idf_test_data(docs_as_tokens);
            let num_docs = test_data.original_df.num_docs as f32;

            // Loop through every token from the original DF map
            for (token, df_score) in &test_data.original_df.map {
                // Get the calculated IDF score
                let idf_score = test_data.idf.0.get(token)
                    .expect("Token from DF map was missing in IDF map");

                // Manually calculate what the score should be
                let expected_idf_val = ((num_docs + 1.0) / (df_score.0 as f32 + 1.0)).ln() + 1.0;

                // Assert that the calculated value is close to the expected value
                prop_assert!(
                    (idf_score - expected_idf_val).abs() < f32::EPSILON,
                    "IDF score for token '{:?}' was calculated incorrectly. Got {}, expected {}",
                    token, idf_score, expected_idf_val
                );
            }
        }

        // Monotonic decreasing relationship
        #[test]
        fn idf_is_inversely_related_to_document_frequency(docs_as_tokens: Vec<Vec<Token<'_>>>) {
            let test_data = setup_idf_test_data(docs_as_tokens);

            // This property is only meaningful if we have at least two different tokens to compare.
            prop_assume!(test_data.original_df.map.len() >= 2);

            let mut sorted_by_df: Vec<_> = test_data.original_df.map.iter().collect();
            // Sort by document frequency (ascending)
            sorted_by_df.sort_by(|a, b| {
                let df_a = a.1.0;
                let df_b = b.1.0;
                df_a.total_cmp(&df_b)
            });

            // The corresponding IDF scores for these tokens must be in descending order.
            let idf_scores_in_df_order: Vec<Score> = sorted_by_df
                .iter()
                .map(|(token, _)| *test_data.idf.0.get(token).unwrap())
                .collect();
            for i in 0..(idf_scores_in_df_order.len() - 1) {
                prop_assert!(
                    idf_scores_in_df_order[i] >= idf_scores_in_df_order[i+1],
                    "IDF scores are not monotonically decreasing. DF {:?} -> IDF {}, but DF {:?} -> IDF {}",
                    sorted_by_df[i].1.0, idf_scores_in_df_order[i],
                    sorted_by_df[i+1].1.0, idf_scores_in_df_order[i+1]
                );
            }
        }
    }
}
