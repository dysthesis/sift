use axum::{
    Router,
    routing::post,
};
use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use libsift::handler::url::handle_url;
 

use crate::cli::Cli;

mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    let cli = Cli::parse();

    // build our application with a route
    let app = Router::new()
        // `POST /users` goes to `create_user`
        .route("/url", post(handle_url));

    let bind_addr = format!("localhost:{}", cli.port);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(bind_addr).await.unwrap();
    axum::serve(listener, app).await.map_err(|e| eyre!(e))
}
