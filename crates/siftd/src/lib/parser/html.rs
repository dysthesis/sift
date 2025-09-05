use bytes::Bytes;
use serde_json::Value;
use tracing::{info, warn};
use url::Url;
use webpage::HTML;

use crate::{
    entry::Entry,
    metadata::Metadata,
    parser::{Parser, ParserFamily},
};

pub struct HtmlParser {
    url: Url,
    content: String,
}
impl<'a> Parser<'a> for HtmlParser {
    fn new(bytes: &Bytes, headers: &'a reqwest::header::HeaderMap, url: &Url) -> Option<Box<Self>>
    where
        Self: Sized,
    {
        if let Some(content_type) = headers.get(reqwest::header::CONTENT_TYPE)
            && let Ok(content_type) = content_type.to_str()
            && content_type.contains("text/html")
            && let Ok(text) = str::from_utf8(bytes)
        {
            let content = text.to_string();
            let url = url.clone();
            Some(Box::new(Self { url, content }))
        } else {
            warn!("Is not HTML.");
            None
        }
    }

    fn parse(&self) -> crate::entry::Entry {
        info!("Parsing {} as HTML...", self.url);
        let html = HTML::from_string(self.content.clone(), Some(self.url.to_string()))
            .expect("HTML parsing should work");
        let title = html.title.unwrap_or_default();
        info!("Found title: {title}");
        let summary = html.description;
        info!("Found summary: {summary:?}");
        let url = self.url.clone();
        info!("Found url: {url}");
        let content = html.text_content;
        info!("Found content: {content}");

        let author = html
            .meta
            .get("author")
            .cloned()
            .or_else(|| html.meta.get("article:author").cloned())
            .or_else(|| html.meta.get("parsely-author").cloned())
            .or_else(|| html.meta.get("dc.creator").cloned())
            .or_else(|| html.meta.get("dcterms.creator").cloned())
            .or_else(|| {
                html.meta
                    .get("twitter:creator")
                    .cloned()
                    .map(|h| h.trim_start_matches('@').to_string())
            })
            .or_else(|| {
                html.schema_org
                    .iter()
                    .find_map(|s| author_from_schema(&s.value))
            })
            .unwrap_or_default();
        info!("Found author: {author}");

        let origin = html
            .opengraph
            .properties
            .get("site_name")
            .cloned()
            .or_else(|| html.meta.get("application-name").cloned())
            .or_else(|| {
                html.meta
                    .get("twitter:site")
                    .cloned()
                    .map(|h| h.trim_start_matches('@').to_string())
            })
            .unwrap_or_else(|| self.url.domain().unwrap_or_default().to_string());
        info!("Found site name: {origin}");

        let metadata = Some(Metadata::new(summary, None, None));

        Entry::new(title, origin, author, url, content, metadata)
    }
}

impl ParserFamily for HtmlParser {
    type For<'a> = HtmlParser;
}

fn author_from_schema(v: &Value) -> Option<String> {
    let a = v.get("author")?;
    match a {
        Value::String(s) => Some(s.clone()),
        Value::Object(m) => m.get("name").and_then(|n| n.as_str()).map(str::to_owned),
        Value::Array(xs) => xs.iter().find_map(|x| {
            x.get("name")
                .and_then(|n| n.as_str())
                .map(str::to_owned)
                .or_else(|| x.as_str().map(str::to_owned))
        }),
        _ => None,
    }
}
