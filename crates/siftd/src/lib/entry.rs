use serde::{Deserialize, Serialize};
use url::Url;

use crate::metadata::Metadata;

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    title: String,
    origin: String,
    author: String,
    url: Url,
    metadata: Metadata,
}
