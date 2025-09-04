use bytes::Bytes;
use url::Url;

use crate::{entry::Entry, parser::html::HtmlParser};

mod html;

pub trait Parser<'a> {
    fn new(bytes: &Bytes, headers: &'a reqwest::header::HeaderMap, url: &Url) -> Option<Box<Self>>
    where
        Self: Sized;
    fn parse(&self) -> Entry;
}

type ParserFn = for<'a> fn(
    &'a Bytes,
    &'a reqwest::header::HeaderMap,
    &'a Url,
) -> Option<Box<dyn Parser<'a> + 'a>>;

/// List of parser constructors to iterate over.
///
/// NOTE: Ordering is important here, as it signifies priority. If two parsers are able to parse a
/// given identifier, the first one to show up in this list will be used.
static PARSERS: &[ParserFn] = &[construct::<HtmlParser>];

// Use GAT because we don't have higher-kinded types in Rust (sad)
pub trait ParserFamily {
    type For<'a>: Parser<'a> + 'a;
}

fn construct<'a, F>(
    bytes: &'a Bytes,
    headers: &'a reqwest::header::HeaderMap,
    url: &'a Url,
) -> Option<Box<dyn Parser<'a> + 'a>>
where
    F: ParserFamily,
{
    <F::For<'a> as Parser<'a>>::new(bytes, headers, url).map(|x| x as Box<dyn Parser<'a> + 'a>)
}

pub fn identify<'a>(
    bytes: &'a Bytes,
    header: &'a reqwest::header::HeaderMap,
    url: &'a Url,
) -> Option<Box<dyn Parser<'a> + 'a>> {
    PARSERS.iter().find_map(|f| f(bytes, header, url))
}
