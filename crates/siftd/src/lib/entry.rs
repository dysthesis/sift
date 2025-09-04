use serde::{Deserialize, Serialize};
use url::Url;

use crate::metadata::Metadata;

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    title: String,
    origin: String,
    author: String,
    url: Url,
    content: String,
    metadata: Metadata,
}

impl Entry {
    pub fn new(
        title: String,
        origin: String,
        author: String,
        url: Url,
        content: String,
        metadata: Option<Metadata>,
    ) -> Self {
        let metadata = metadata.unwrap_or_default();
        Self {
            title,
            origin,
            author,
            url,
            content,
            metadata,
        }
    }
}
