use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

type Time = DateTime<Utc>;
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    published_time: Option<Time>,

    #[serde(skip_serializing_if = "Option::is_none")]
    updated_time: Option<Time>,
}
