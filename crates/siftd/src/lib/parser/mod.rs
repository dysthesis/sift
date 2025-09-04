use crate::entry::Entry;

pub trait Parser {
    fn parse(content: &[u8]) -> Entry
    where
        Self: Sized;
}
