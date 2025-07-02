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
    struct TestSetupData<'a> {
        actual_df: Df<'a>,
        actual_idf: Idf<'a>,
        ground_truth_df: HashMap<Token<'a>, u32>,
    }

    fn setup_test_data(docs_as_tokens: Vec<Vec<Token<'_>>>) -> TestSetupData<'_> {
        let tfs: Vec<Tf> = docs_as_tokens
            .iter()
            .map(|doc| Tf::from(doc.iter().copied()))
            .collect();

        let mut ground_truth_df_map: HashMap<Token, u32> = HashMap::new();
        for tf in &tfs {
            // Iterate through the unique tokens of each document.
            for token in tf.borrow_map().keys() {
                *ground_truth_df_map.entry(*token).or_insert(0) += 1;
            }
        }

        let actual_df = Df::from(tfs.as_slice());
        // We must clone because Idf::from consumes its input.
        let actual_idf = Idf::from(actual_df.clone());

        TestSetupData {
            actual_df,
            actual_idf,
            ground_truth_df: ground_truth_df_map,
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn df_num_docs_is_correct(docs: Vec<Vec<Token<'_>>>) {
            let num_docs = docs.len();
            let test_data = setup_test_data(docs);

            prop_assert_eq!(test_data.actual_df.num_docs, num_docs);
        }

        #[test]
        fn df_unique_token_count_is_correct(docs: Vec<Vec<Token<'_>>>) {
            let test_data = setup_test_data(docs);
            prop_assert_eq!(test_data.actual_df.map.len(), test_data.ground_truth_df.len());
        }

        #[test]
        fn df_score_is_correct(docs: Vec<Vec<Token<'_>>>) {
            let test_data = setup_test_data(docs);

            for (token, &expected_count) in &test_data.ground_truth_df {
                let df_score = test_data.actual_df.map.get(token)
                    .expect("Token from ground truth missing in actual result");

                prop_assert_eq!(df_score.0, expected_count as f32);
            }
        }

        #[test]
        fn idf_map_preserves_all_tokens(docs: Vec<Vec<Token<'_>>>) {
            let test_data = setup_test_data(docs);

            prop_assert_eq!(test_data.actual_idf.0.len(), test_data.ground_truth_df.len());
        }

        #[test]
        fn idf_score_is_always_positive_and_smoothed(docs: Vec<Vec<Token<'_>>>) {
            let test_data = setup_test_data(docs);

            for idf_score in test_data.actual_idf.0.values() {
                prop_assert!(*idf_score >= 1.0);
            }
        }

        #[test]
        fn idf_score_is_calculated_correctly(docs: Vec<Vec<Token<'_>>>) {
            let test_data = setup_test_data(docs);
            let num_docs = test_data.actual_df.num_docs as f32;

            for (token, &expected_df_count) in &test_data.ground_truth_df {
                let idf_score = test_data.actual_idf.0.get(token).unwrap();
                let expected_idf_val = ((num_docs + 1.0) / (expected_df_count as f32 + 1.0)).ln() + 1.0;

                prop_assert!((idf_score - expected_idf_val).abs() < f32::EPSILON);
            }
        }

        #[test]
        fn idf_is_inversely_related_to_document_frequency(docs: Vec<Vec<Token<'_>>>) {
            let test_data = setup_test_data(docs);
            prop_assume!(test_data.ground_truth_df.len() >= 2);

            let mut sorted_by_df: Vec<_> = test_data.ground_truth_df.iter().collect();
            sorted_by_df.sort_by_key(|&(_token, count)| count);

            let idf_scores_in_df_order: Vec<Score> = sorted_by_df
                .iter()
                .map(|(token, _)| *test_data.actual_idf.0.get(token).unwrap())
                .collect();

            for i in 0..(idf_scores_in_df_order.len() - 1) {
                prop_assert!(idf_scores_in_df_order[i] >= idf_scores_in_df_order[i+1]);
            }
        }
    }
}
