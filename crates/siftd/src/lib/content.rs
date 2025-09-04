use std::marker::PhantomData;

use bytes::Bytes;
use thiserror::Error;
use url::Url;

use crate::{
    entry::Entry,
    metadata::Metadata,
    parser::{identify, Parser},
    HTTP_CLIENT,
};

pub trait ContentState {}
pub struct Content<'a, S>
where
    S: ContentState,
{
    url: Url,
    bytes: Option<Bytes>,
    headers: Option<reqwest::header::HeaderMap>,
    parser: Option<Box<dyn Parser<'a> + 'a>>,
    metadata: Metadata,
    _state: PhantomData<S>,
}

impl<'a, S> Content<'a, S>
where
    S: ContentState,
{
    pub fn new(url: Url, metadata: Option<Metadata>) -> Content<'a, Unfetched> {
        let metadata = metadata.unwrap_or_default();
        Content {
            url,
            bytes: None,
            headers: None,
            parser: None,
            metadata,
            _state: PhantomData::<Unfetched>,
        }
    }
}

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("Failed to fetch URL {url}: {error}")]
    FetchError { error: reqwest::Error, url: String },
    #[error("Failed to parse body. URL: {url}")]
    ParseError { url: String },
}

pub struct Unfetched;
impl ContentState for Unfetched {}
impl<'a> Content<'a, Unfetched> {
    pub async fn fetch(self) -> Result<Content<'a, Fetched>, ContentError> {
        let raw_response = HTTP_CLIENT
            .get(self.url.as_str())
            .send()
            .await
            .map_err(|error| ContentError::FetchError {
                error,
                url: self.url.to_string(),
            })?;

        // TODO: Can this be destructured instead to prevent cloning?
        let headers = raw_response.headers().clone();

        let bytes = raw_response
            .bytes()
            .await
            .map_err(|error| ContentError::FetchError {
                error,
                url: self.url.to_string(),
            })?;

        // let parser = identify(&bytes, &headers);

        let Content {
            url,
            parser,
            metadata,
            _state: _,
            bytes: _,
            headers: _,
        } = self;

        Ok(Content {
            headers: Some(headers),
            bytes: Some(bytes),
            _state: PhantomData::<Fetched>,
            url,
            parser,
            metadata,
        })
    }
}

pub struct Fetched;
impl ContentState for Fetched {}
impl<'a> Content<'a, Fetched> {
    pub fn parse(self) -> Result<Entry, ContentError> {
        let entry = self
            .parser
            .ok_or_else(|| ContentError::ParseError {
                url: self.url.to_string(),
            })?
            .parse();
        Ok(entry)
    }
}
