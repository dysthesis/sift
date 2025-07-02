use std::collections::HashMap;

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::similarity::{
    Score,
    idf::{Df, Idf},
    tf::Tf,
    token::Token,
};

pub struct TfIdf<'a>(Vec<HashMap<Token<'a>, Score>>);
impl<'a> TfIdf<'a> {
    pub fn get(&self) -> &Vec<HashMap<Token<'a>, Score>> {
        &self.0
    }
}
impl<'a> From<&[&[Token<'a>]]> for TfIdf<'a> {
    fn from(corpus: &[&[Token<'a>]]) -> Self {
        let tf: Vec<Tf<'a>> = corpus
            .par_iter()
            .map(|doc| Tf::from(doc.iter().copied()))
            .collect();
        let idf: Idf = Df::from(tf.as_slice()).into();
        let res = tf
            .par_iter()
            .map(|val| {
                val.borrow_map()
                    .into_par_iter()
                    .filter_map(|(term, &tf_w)| {
                        let tf_w: Score = tf_w.into();
                        idf.get(term).map(|idf_w| (term, tf_w * idf_w))
                    })
                    .map(|(k, v)| (*k, v))
                    .collect::<HashMap<_, _>>()
            })
            .collect();
        Self(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    struct TfIdfTestData<'a> {
        result_tfidf: TfIdf<'a>,
        tfs: Vec<Tf<'a>>,
        idf: Idf<'a>,
        corpus: Vec<Vec<Token<'a>>>,
    }

    fn setup_tfidf_test_data(corpus: Vec<Vec<Token<'_>>>) -> TfIdfTestData<'_> {
        let tfs: Vec<Tf> = corpus
            .iter()
            .map(|doc| Tf::from(doc.iter().copied()))
            .collect();
        let idf: Idf = Df::from(tfs.as_slice()).into();

        let corpus_slices: Vec<&[Token]> = corpus.iter().map(AsRef::as_ref).collect();

        let result_tfidf = TfIdf::from(corpus_slices.as_slice());

        TfIdfTestData {
            result_tfidf,
            tfs,
            idf,
            corpus,
        }
    }
    proptest! {
        #[test]
        fn tfidf_preserves_document_count(
            corpus in proptest::collection::vec(proptest::collection::vec(any::<Token>(), 0..50), 0..10)
        ) {
            let test_data = setup_tfidf_test_data(corpus);
            prop_assert_eq!(
                test_data.result_tfidf.get().len(),
                test_data.corpus.len(),
                "The number of documents in the output should match the input corpus size"
            );
        }
    }

    proptest! {
        #[test]
        fn tfidf_preserves_tokens_within_each_document(
             corpus in proptest::collection::vec(proptest::collection::vec(any::<Token>(), 0..50), 0..10)
        ) {
            let test_data = setup_tfidf_test_data(corpus);
            let result_docs = test_data.result_tfidf.get();

            for i in 0..test_data.corpus.len() {
                let original_tokens: HashSet<Token> = test_data.corpus[i].iter().copied().collect();
                let result_tokens: HashSet<Token> = result_docs[i].keys().copied().collect();

                prop_assert_eq!(
                    result_tokens,
                    original_tokens,
                    "Token set for document {} should be identical to the original", i
                );
            }
        }
    }

    proptest! {
        #[test]
        fn tfidf_scores_are_always_positive(
            corpus in proptest::collection::vec(proptest::collection::vec(any::<Token>(), 0..50), 0..10)
        ) {
            let test_data = setup_tfidf_test_data(corpus);

            for doc_map in test_data.result_tfidf.get() {
                for (token, score) in doc_map {
                    prop_assert!(
                        *score > 0.0,
                        "TF-IDF score for token {:?} was {}, but should be positive",
                        token, score
                    );
                }
            }
        }
    }

    proptest! {
        #[test]
        fn tfidf_score_is_calculated_correctly(
            corpus in proptest::collection::vec(proptest::collection::vec(any::<Token>(), 0..50), 0..10)
        ) {
            let test_data = setup_tfidf_test_data(corpus);
            let result_docs = test_data.result_tfidf.get();

            for i in 0..test_data.corpus.len() {
                let tf_map = test_data.tfs[i].borrow_map();
                let result_map = &result_docs[i];

                for (token, tf_score) in tf_map {
                    let actual_tfidf_score = result_map.get(token)
                        .expect("Token from TF map was missing in final result");

                    let idf_score = test_data.idf.get(token)
                        .expect("Token from TF map was missing in IDF map");

                    let tf_val: f32 = tf_score.clone().into();
                    let expected_tfidf_score = tf_val * idf_score;

                    prop_assert!(
                        (actual_tfidf_score - expected_tfidf_score).abs() < f32::EPSILON,
                        "Incorrect TF-IDF for token {:?} in doc {}: got {}, expected {}",
                        token, i, actual_tfidf_score, expected_tfidf_score
                    );
                }
            }
        }
    }
}
