use once_cell::sync::Lazy;
use reqwest::Client;

mod content;
mod entry;
mod metadata;
mod parser;

pub static HTTP_CLIENT: Lazy<Client> = Lazy::new(Client::new);
