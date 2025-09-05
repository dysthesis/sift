use bytes::Bytes;
use mime::Mime;
use serde_json::Value;
use tracing::{debug, info};
use url::Url;
use webpage::HTML;

use crate::{
    entry::Entry,
    metadata::Metadata,
    parser::{Parser, ParserError, ParserFamily},
};

pub struct HtmlParser {
    url: Url,
    bytes: Bytes,
    content_type: Option<Mime>,
}
impl<'a> Parser<'a> for HtmlParser {
    fn new(bytes: &Bytes, headers: &'a reqwest::header::HeaderMap, url: &Url) -> Option<Box<Self>>
    where
        Self: Sized,
    {
        // Parse MIME from header, accepting text/html and application/xhtml+xml
        let mime = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<Mime>().ok());

        let is_html_mime = mime.as_ref().is_some_and(|m| {
            (m.type_() == mime::TEXT && m.subtype() == mime::HTML)
                || m.essence_str()
                    .eq_ignore_ascii_case("application/xhtml+xml")
        });

        if is_html_mime || looks_like_html(bytes) {
            Some(Box::new(Self {
                url: url.clone(),
                bytes: bytes.clone(),
                content_type: mime,
            }))
        } else {
            debug!("Body does not look like HTML; skipping HtmlParser.");
            None
        }
    }

    fn parse(&self) -> Result<crate::entry::Entry, ParserError> {
        info!("Parsing {} as HTML...", self.url);

        // Decode to UTF-8 String. Prefer header charset if present; otherwise fall back to lossy UTF-8.
        let decoded = match charset_from_mime(self.content_type.as_ref()) {
            Some(cs) => decode_with_charset(&self.bytes, &cs),
            None => String::from_utf8_lossy(&self.bytes).into_owned(),
        };

        debug!("Decoded HTML length: {} bytes", decoded.len());

        let html = HTML::from_string(decoded.clone(), Some(self.url.to_string()))
            .map_err(|e| ParserError::WebpageParse(anyhow::Error::new(e)))?;

        // Build a `scraper` DOM for heuristics.
        let document = scraper::Html::parse_document(&decoded);

        let title = pick_title(&html, &document);
        debug!(
            "Title chosen (len={}): {}",
            title.len(),
            truncate_for_log(&title)
        );

        let mut summary = html
            .opengraph
            .properties
            .get("description")
            .cloned()
            .or_else(|| html.meta.get("twitter:description").cloned())
            .or(html.description.clone())
            .filter(|s| !s.trim().is_empty());

        // Main content extraction with heuristics; fallback to webpage text_content
        let content = extract_main_content(&document)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| html.text_content.clone());

        if summary.is_none() {
            summary = content
                .split('\n')
                .find(|p| !p.trim().is_empty())
                .map(|s| s.trim().to_string());
        }

        let author = pick_author(&html).unwrap_or_default();
        let origin = pick_origin(&html, &self.url);
        let (published_time, updated_time) = extract_times(&html);

        let url = self.url.clone();
        let content_capped = cap_len(content, 400_000);

        let metadata = Some(Metadata::new(summary, published_time, updated_time));

        Ok(Entry::new(
            title,
            origin,
            author,
            url,
            content_capped,
            metadata,
        ))
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

fn truncate_for_log(s: &str) -> String {
    const MAX: usize = 120;
    if s.len() <= MAX {
        s.to_string()
    } else {
        format!("{}…", &s[..MAX])
    }
}

fn looks_like_html(bytes: &Bytes) -> bool {
    let probe = &bytes[..bytes.len().min(2048)];
    let lower = probe
        .iter()
        .map(|b| b.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let hay = lower.as_slice();
    hay.windows(14).any(|w| w == b"<!doctype html".as_ref())
        || hay.windows(5).any(|w| w == b"<html".as_ref())
}

fn charset_from_mime(mime: Option<&Mime>) -> Option<String> {
    mime.and_then(|m| m.get_param(mime::CHARSET).map(|v| v.to_string()))
}

fn decode_with_charset(bytes: &Bytes, charset: &str) -> String {
    // Basic decoding: prefer utf-8; otherwise fall back to lossily mapping bytes.
    if charset.eq_ignore_ascii_case("utf-8") || charset.eq_ignore_ascii_case("utf8") {
        match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(bytes).into_owned(),
        }
    } else {
        // If not UTF-8, defer to lossy conversion to ensure robustness without panicking.
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn pick_title(html: &HTML, doc: &scraper::Html) -> String {
    html.opengraph
        .properties
        .get("title")
        .cloned()
        .or_else(|| html.meta.get("twitter:title").cloned())
        .or(html.title.clone())
        .or_else(|| first_text_selector(doc, "h1"))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn first_text_selector(doc: &scraper::Html, selector: &str) -> Option<String> {
    let sel = scraper::Selector::parse(selector).ok()?;
    let node = doc.select(&sel).next()?;
    Some(node.text().collect::<Vec<_>>().join(" ").trim().to_string())
}

fn extract_main_content(doc: &scraper::Html) -> Option<String> {
    // Candidate containers
    let candidates = [
        "article",
        "main",
        "[role=main]",
        "#content",
        "#main",
        ".post-content",
        ".article-content",
        ".article-body",
        ".entry-content",
        "[itemprop=articleBody]",
    ];

    for sel in candidates
        .iter()
        .filter_map(|s| scraper::Selector::parse(s).ok())
    {
        if let Some(container) = doc.select(&sel).next() {
            let blocks =
                scraper::Selector::parse("p, h1, h2, h3, h4, h5, h6, li, blockquote, pre").ok()?;
            let mut parts = Vec::new();
            for node in container.select(&blocks) {
                let t = node.text().collect::<Vec<_>>().join(" ");
                let t = t.trim();
                if !t.is_empty() {
                    parts.push(t.to_string());
                }
            }
            let joined = parts.join("\n\n");
            if !joined.trim().is_empty() {
                return Some(joined);
            }
        }
    }
    None
}

fn pick_author(html: &HTML) -> Option<String> {
    let meta_candidate = html
        .meta
        .get("author")
        .cloned()
        .or_else(|| html.meta.get("article:author").cloned())
        .or_else(|| html.meta.get("parsely-author").cloned())
        .or_else(|| html.meta.get("dc.creator").cloned())
        .or_else(|| html.meta.get("dcterms.creator").cloned())
        .or_else(|| html.meta.get("byline").cloned())
        .or_else(|| html.meta.get("byl").cloned())
        .or_else(|| {
            html.meta
                .get("twitter:creator")
                .cloned()
                .map(|h| h.trim_start_matches('@').to_string())
        });

    if meta_candidate.is_some() {
        return meta_candidate.map(|s| s.trim().to_string());
    }

    if let Some(from_schema) = html
        .schema_org
        .iter()
        .find_map(|s| author_from_schema(&s.value))
    {
        return Some(from_schema.trim().to_string());
    }

    // OpenGraph article:author can be present in opengraph properties
    if let Some(a) = html.opengraph.properties.get("article:author").cloned() {
        return Some(a.trim().to_string());
    }

    None
}

fn pick_origin(html: &HTML, url: &Url) -> String {
    html.opengraph
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
        .or_else(|| url.domain().map(|d| d.to_string()))
        .unwrap_or_default()
}

fn extract_times(
    html: &HTML,
) -> (
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
) {
    use chrono::{DateTime, Utc};

    let mut published: Option<DateTime<Utc>> = None;
    let mut updated: Option<DateTime<Utc>> = None;

    // OpenGraph
    if let Some(v) = html.opengraph.properties.get("article:published_time")
        && let Some(dt) = parse_time(v)
    {
        published = Some(dt);
    }
    if let Some(v) = html.opengraph.properties.get("article:modified_time")
        && let Some(dt) = parse_time(v)
    {
        updated = Some(dt);
    }
    if let Some(v) = html.opengraph.properties.get("updated_time")
        && let Some(dt) = parse_time(v)
    {
        updated = Some(dt);
    }

    // Schema.org
    for s in &html.schema_org {
        if published.is_none()
            && let Some(v) = s.value.get("datePublished").and_then(|x| x.as_str())
            && let Some(dt) = parse_time(v)
        {
            published = Some(dt);
        }
        if updated.is_none()
            && let Some(v) = s.value.get("dateModified").and_then(|x| x.as_str())
            && let Some(dt) = parse_time(v)
        {
            updated = Some(dt);
        }
        if published.is_some() && updated.is_some() {
            break;
        }
    }

    (published, updated)
}

fn parse_time(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Common fallback formats
    let fmts = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%d"];
    for f in fmts {
        if f == "%Y-%m-%d" {
            debug!("Format for {s} is %Y-%m-%d");
            if let Ok(d) = NaiveDate::parse_from_str(s, f) {
                return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
            }
        } else if let Ok(ndt) = NaiveDateTime::parse_from_str(s, f) {
            return Some(chrono::DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc));
        }
    }
    None
}

fn cap_len(s: String, max: usize) -> String {
    if s.len() <= max {
        s
    } else {
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}
