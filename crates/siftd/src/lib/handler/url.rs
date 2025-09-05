use axum::{http::StatusCode, Json};
use serde::Deserialize;
use tracing::info;
use url::Url;

use crate::{
    content::{Content, Unfetched},
    entry::Entry,
};

pub async fn handle_url(Json(payload): Json<HandleUrl>) -> Result<(StatusCode, Json<Entry>), (StatusCode, String)> {
    let url = payload.url;
    let (url_host, url_path) = crate::url_host_and_path(&url);
    info!(%url_host, %url_path, "process url");
    let content = Content::<Unfetched>::new(url, None);
    let entry = content
        .fetch()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("fetch error: {e}")))?
        .parse()
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, format!("parse error: {e}")))?;
    Ok((StatusCode::CREATED, Json(entry)))
}

#[derive(Deserialize, Debug)]
pub struct HandleUrl {
    url: Url,
}
