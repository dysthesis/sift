use bytes::Bytes;
use scraper::{Html, Selector};
use url::Url;

use crate::{
    entry::Entry,
    parser::{Parser, ParserFamily},
};

pub struct HtmlParser<'a> {
    url: Url,
    headers: &'a reqwest::header::HeaderMap,
    content: String,
}
impl<'a> Parser<'a> for HtmlParser<'a> {
    fn new(bytes: &Bytes, headers: &'a reqwest::header::HeaderMap, url: &Url) -> Option<Box<Self>>
    where
        Self: Sized,
    {
        if let Some(content_type) = headers.get(reqwest::header::CONTENT_TYPE)
            && let Ok(content_type) = content_type.to_str()
            && content_type == "text/html"
            && let Ok(text) = str::from_utf8(bytes)
        {
            let content = text.to_string();
            let url = url.clone();
            Some(Box::new(Self {
                url,
                headers,
                content,
            }))
        } else {
            None
        }
    }

    fn parse(&self) -> crate::entry::Entry {
        let url = self.url.clone();
        let doc = Html::parse_document(self.content.as_str());

        let selector_title = Selector::parse("title").unwrap();
        let selector_og_title = Selector::parse(r#"meta[property="og:title"]"#).unwrap();
        let selector_site = Selector::parse(r#"meta[property="og:site_name"]"#).unwrap();
        let selector_author = Selector::parse(
            r#"meta[name="author"],
               meta[property="article:author"],
               meta[name="byl"],
               meta[name="dc.creator"],
               meta[name="parsely-author"],
               meta[name="twitter:creator"]"#,
        )
        .unwrap();

        let title = doc
            .select(&selector_og_title)
            .next()
            .and_then(|m| m.value().attr("content"))
            .map(str::to_owned)
            .or_else(|| {
                doc.select(&selector_title)
                    .next()
                    .map(|n| n.text().collect::<String>().trim().to_owned())
            })
            .unwrap_or_else(|| url.as_str().to_owned());

        let origin = doc
            .select(&selector_site)
            .next()
            .and_then(|m| m.value().attr("content"))
            .map(str::to_owned)
            .or_else(|| url.host_str().map(|h| h.to_string()))
            .unwrap_or_default();

        let author = doc
            .select(&selector_author)
            .filter_map(|m| m.value().attr("content"))
            .map(str::trim)
            .find(|s| !s.is_empty())
            .unwrap_or("")
            .to_owned();

        Entry::new(title, origin, author, url, self.content.clone(), None)
    }
}

impl ParserFamily for HtmlParser<'_> {
    type For<'a> = HtmlParser<'a>;
}
