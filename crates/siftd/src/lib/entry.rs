use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

type Time = DateTime<Utc>;
#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    title: String,
    origin: String,
    author: String,
    url: Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    published_time: Option<Time>,

    #[serde(skip_serializing_if = "Option::is_none")]
    updated_time: Option<Time>,
}
