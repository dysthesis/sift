use std::collections::HashMap;

use crate::similarity::{Score, token::Token};

#[derive(Copy, Clone, Default, Debug, PartialEq, PartialOrd)]
pub struct TfScore(Score);
impl From<TfScore> for Score {
    fn from(TfScore(value): TfScore) -> Self {
        value
    }
}

#[derive(Debug)]
pub struct Tf<'a>(HashMap<Token<'a>, TfScore>);
impl<'a, T> From<T> for Tf<'a>
where
    T: Iterator<Item = Token<'a>>,
{
    fn from(terms: T) -> Self {
        let (lower, _upper) = terms.size_hint();
        let tf = terms.fold(
            HashMap::with_capacity(lower),
            |mut frequencies: HashMap<Token<'a>, TfScore>, term| {
                frequencies.entry(term).or_default().0 += 1 as Score;
                frequencies
            },
        );
        Self(tf)
    }
}

impl<'a> Tf<'a> {
    pub fn borrow_map(&self) -> &HashMap<Token<'a>, TfScore> {
        &self.0
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
        fn conservation_of_counts(
            tokens: Vec<Token>
        ) {
            let tf = Tf::from(tokens.iter().copied());

            // The sum of all counts in the map must equal the total number of tokens.
            let sum_of_frequencies: Score = tf.0.values().map(|tf_score| tf_score.0).sum();
            prop_assert_eq!(sum_of_frequencies, tokens.len() as Score, "Sum of frequencies should equal total token count");
        }
        #[test]
        fn unique_term_correspondence(
            tokens: Vec<Token>
        ) {
            let tf = Tf::from(tokens.iter().copied());
            // --- Assert Property 2: Unique Term Correspondence ---
            // The number of entries in the map must equal the number of unique tokens.
            let unique_tokens: HashSet<Token> = tokens.iter().copied().collect();
            prop_assert_eq!(tf.0.len(), unique_tokens.len(), "Map should have one entry per unique token");
        }
        #[test]
        fn positive_frequencies(
            tokens: Vec<Token>
        ) {
            let tf = Tf::from(tokens.iter().copied());
            let unique_tokens: HashSet<Token> = tokens.iter().copied().collect();
            for token in unique_tokens {
                // Check that every unique token from the input exists as a key in the map.
                let frequency = tf.0.get(&token).expect("Unique token not found in Tf map");

                // Check that its count is correct.
                let expected_count = tokens.iter().filter(|&t| *t == token).count() as Score;
                prop_assert_eq!(frequency.0, expected_count, "Frequency count for token is incorrect");

                // This also implicitly checks that the count is > 0, since a token
                // must appear at least once to be in the `unique_tokens` set.
                prop_assert!(frequency.0 > 0 as Score, "Frequency should be positive");
            }
        }
    }
}
