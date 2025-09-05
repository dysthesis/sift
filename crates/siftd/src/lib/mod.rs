use once_cell::sync::Lazy;
use reqwest::Client;

pub mod content;
pub mod entry;
pub mod handler;
pub mod metadata;
pub mod parser;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.10 Safari/605.1.1";
pub static HTTP_CLIENT: Lazy<Client> =
    Lazy::new(|| Client::builder().user_agent(USER_AGENT).build().unwrap());

/// Return host and path of a URL, with query/fragment stripped.
pub fn url_host_and_path(u: &url::Url) -> (String, String) {
    let host = u.host_str().unwrap_or("").to_string();
    (host, u.path().to_string())
}
