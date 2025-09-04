use axum::{http::StatusCode, Json};
use serde::Deserialize;
use tracing::info;
use url::Url;

use crate::{
    content::{Content, Unfetched},
    entry::Entry,
};

pub async fn handle_url(Json(payload): Json<HandleUrl>) -> (StatusCode, Json<Entry>) {
    let url = payload.url;
    info!("Fetching URL {url}");
    let content = Content::<Unfetched>::new(url, None);
    let entry = content
        .fetch()
        .await
        .expect("Fetching to work")
        .parse()
        .inspect_err(|e| eprint!("Error: {e}"))
        .expect("Parsing to work");
    (StatusCode::CREATED, Json(entry))
}

#[derive(Deserialize, Debug)]
pub struct HandleUrl {
    url: Url,
}
