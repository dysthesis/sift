use std::marker::PhantomData;

use bytes::Bytes;
use thiserror::Error;
use tracing::{debug, info, instrument};
use url::Url;

use crate::{entry::Entry, metadata::Metadata, parser::identify, HTTP_CLIENT};

pub trait ContentState {}
pub struct Content<S>
where
    S: ContentState,
{
    url: Url,
    bytes: Option<Bytes>,
    headers: Option<reqwest::header::HeaderMap>,
    metadata: Metadata,
    _state: PhantomData<S>,
}

impl<S> Content<S>
where
    S: ContentState,
{
    pub fn new(url: Url, metadata: Option<Metadata>) -> Content<Unfetched> {
        let metadata = metadata.unwrap_or_default();
        Content {
            url,
            bytes: None,
            headers: None,
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
    ParseError {
        url: String,
        #[source]
        source: anyhow::Error,
    },
}

pub struct Unfetched;
impl ContentState for Unfetched {}
impl Content<Unfetched> {
    #[instrument(level = "info", skip(self), fields(url_host, url_path))]
    pub async fn fetch(self) -> Result<Content<Fetched>, ContentError> {
        let (host, path) = crate::url_host_and_path(&self.url);
        tracing::Span::current().record("url_host", tracing::field::display(&host));
        tracing::Span::current().record("url_path", tracing::field::display(&path));
        info!(%host, %path, "fetch start");
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

        let Content {
            url,
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
            metadata,
        })
    }
}

pub struct Fetched;
impl ContentState for Fetched {}
impl Content<Fetched> {
    #[instrument(level = "info", skip(self), fields(url_host, url_path))]
    pub fn parse(self) -> Result<Entry, ContentError> {
        let (host, path) = crate::url_host_and_path(&self.url);
        tracing::Span::current().record("url_host", tracing::field::display(&host));
        tracing::Span::current().record("url_path", tracing::field::display(&path));
        let bytes = self
            .bytes
            .as_ref()
            .ok_or_else(|| ContentError::ParseError {
                url: self.url.to_string(),
                source: anyhow::anyhow!("missing bytes"),
            })?;
        debug!(bytes_len = bytes.len());

        let headers = self
            .headers
            .as_ref()
            .ok_or_else(|| ContentError::ParseError {
                url: self.url.to_string(),
                source: anyhow::anyhow!("missing headers"),
            })?;
        debug!(headers_present = !headers.is_empty());

        if let Some(parser) = identify(bytes, headers, &self.url) {
            let entry = parser.parse().map_err(|e| ContentError::ParseError {
                url: self.url.to_string(),
                source: anyhow::Error::from(e),
            })?;
            Ok(entry)
        } else {
            Err(ContentError::ParseError {
                url: self.url.to_string(),
                source: anyhow::anyhow!("no suitable parser"),
            })
        }
    }
}
