use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

type Time = DateTime<Utc>;
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail_url: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    published_time: Option<Time>,

    #[serde(skip_serializing_if = "Option::is_none")]
    updated_time: Option<Time>,
}

impl Metadata {
    pub fn new(
        summary: Option<String>,
        published_time: Option<Time>,
        updated_time: Option<Time>,
        thumbnail_url: Option<Url>,
    ) -> Self {
        Self {
            summary,
            published_time,
            updated_time,
            thumbnail_url,
        }
    }
}
