use std::{collections::HashMap, hash::Hash};

use crate::similarity::Score;

/// Computes the cosine similarity between two sparse vectors.
pub fn cosine_similarity_sparse<T>(a: &HashMap<T, Score>, b: &HashMap<T, Score>) -> Score
where
    T: Eq + Hash,
{
    fn dot<U: Eq + Hash>(v1: &HashMap<U, Score>, v2: &HashMap<U, Score>) -> Score {
        let (short, long) = if v1.len() < v2.len() {
            (v1, v2)
        } else {
            (v2, v1)
        };
        short
            .iter()
            .filter_map(|(key, &val1)| long.get(key).map(|&val2| val1 * val2))
            .sum()
    }

    let norm_a = a.values().map(|&x| x * x).sum::<Score>().sqrt();
    let norm_b = b.values().map(|&x| x * x).sum::<Score>().sqrt();

    let denom = norm_a * norm_b;
    if denom == 0.0 { 0.0 } else { dot(a, b) / denom }
}
#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;
    use std::collections::HashMap;

    // Let's assume Score is f32 for this test, as used in the function.
    // In a real system, your Score wrapper would be used here.
    type Score = f32;

    // Helper function to create a strategy for generating sparse vectors.
    // We'll generate non-negative scores, typical for TF-IDF.
    fn sparse_vector_strategy() -> impl Strategy<Value = HashMap<String, Score>> {
        // A map of 0 to 50 entries.
        proptest::collection::hash_map(
            // Keys are short alphabetic strings.
            "[a-z]{1,5}",
            // Values are non-negative floats.
            0.0f32..1000.0,
            0..50,
        )
    }

    // Floating point arithmetic is inexact, so we need a small tolerance.
    const EPSILON: f32 = 1e-6;
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100_000))]
        #[test]
        fn cosine_similarity_is_between_zero_and_one(
            a in sparse_vector_strategy(),
            b in sparse_vector_strategy()
        ) {
            let sim = cosine_similarity_sparse(&a, &b);

            prop_assert!(sim >= 0.0 && sim <= 1.0 + EPSILON,
                "Similarity {} was outside the range [0, 1]", sim);
        }

        #[test]
        fn cosine_similarity_is_symmetric(
            a in sparse_vector_strategy(),
            b in sparse_vector_strategy()
        ) {
            let sim_ab = cosine_similarity_sparse(&a, &b);
            let sim_ba = cosine_similarity_sparse(&b, &a);

            prop_assert!((sim_ab - sim_ba).abs() < EPSILON,
                "Similarity should be symmetric: sim(a,b)={}, sim(b,a)={}", sim_ab, sim_ba);
        }

        #[test]
        fn similarity_of_a_vector_with_itself_is_one(
            a in sparse_vector_strategy().prop_filter(
                "Vector must be non-zero",
                |v| v.values().map(|&x| x*x).sum::<Score>() > 0.0
            )
        ) {
            // prop_assume! is no longer needed.
            let sim = cosine_similarity_sparse(&a, &a);
            prop_assert!((sim - 1.0).abs() < EPSILON);
        }

        #[test]
        fn similarity_is_invariant_to_positive_scaling(
            a in sparse_vector_strategy(),
            b in proptest::collection::hash_map(
                    "[a-z]{1,5}",
                    0.0f32..1000.0,
                    1..50, // This guarantees the map is not empty
                ),
            c in 1.0f32..100.0,
        ) {
            let b_scaled: HashMap<String, Score> = b.iter().map(|(k, v)| (k.clone(), v * c)).collect();
            let sim_original = cosine_similarity_sparse(&a, &b);
            let sim_scaled = cosine_similarity_sparse(&a, &b_scaled);
            prop_assert!((sim_original - sim_scaled).abs() < EPSILON);
        }

        #[test]
        fn similarity_of_orthogonal_vectors_is_zero(
            mut a in sparse_vector_strategy(),
            b in sparse_vector_strategy().prop_filter(
                "Vector must be non-zero",
                |v| v.values().any(|&s| s > 0.0)
            )
        ) {
            for k in b.keys() {
                a.remove(k);
            }

            let sim = cosine_similarity_sparse(&a, &b);
            prop_assert!((sim - 0.0).abs() < EPSILON);
        }
    }

    #[test]
    fn cosine_similarity_of_zero_vectors() {
        let a: HashMap<String, Score> = HashMap::new();
        let b: HashMap<String, Score> = HashMap::new();
        assert_eq!(cosine_similarity_sparse(&a, &b), 0.0);

        let mut c: HashMap<String, Score> = HashMap::new();
        c.insert("hello".to_string(), 1.0);
        assert_eq!(cosine_similarity_sparse(&a, &c), 0.0);
        assert_eq!(cosine_similarity_sparse(&c, &a), 0.0);
    }
}
