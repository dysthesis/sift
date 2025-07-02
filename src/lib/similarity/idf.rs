use std::collections::HashMap;

use crate::similarity::{Score, token::Token};

pub struct DfScore(Score);

pub struct Df<'a> {
    map: HashMap<Token<'a>, DfScore>,
    num_docs: usize,
}
