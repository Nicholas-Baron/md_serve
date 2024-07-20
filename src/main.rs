use std::path::{Path, PathBuf};

use axum::extract::{Path as URLPath, State};
use axum::response::{self, Html, IntoResponse, Response};
use axum::routing::get;
use axum::{http::header, Router};
use tokio::net::TcpListener;
use tokio::{fs, io};

use tracing::info;

mod configuration;
use configuration::Configuration;

mod html_cache;
use html_cache::HTMLCache;

async fn serve_raw_resource(filename: &Path) -> io::Result<impl IntoResponse> {
    let file = fs::read(filename).await?;

    let mimetype = mime_guess::from_path(filename).first().unwrap();

    let headers = [(header::CONTENT_TYPE, mimetype.to_string())];

    Ok((headers, file))
}

async fn serve_path(
    URLPath(path): URLPath<String>,
    State(html_cache): State<HTMLCache>,
) -> response::Result<Response> {
    let mut local_filename = PathBuf::from(path.clone());
    if local_filename.extension().is_none() {
        local_filename.set_extension("md");

        info!(
            "{path} requested. Sourcing from {}",
            local_filename.display()
        );

        let output_html = html_cache
            .cache_markdown(&local_filename)
            .await
            .map_err(|e| {
                format!(
                    "Error creating cached version of {}: {e}",
                    local_filename.display()
                )
            })?;

        Ok(fs::read_to_string(&output_html)
            .await
            .map(Html)
            .map(IntoResponse::into_response)
            .map_err(|e| format!("Error reading from {}: {e}", output_html.display()))?)
    } else {
        Ok(serve_raw_resource(&local_filename)
            .await
            .map(IntoResponse::into_response)
            .map_err(|e| {
                format!(
                    "Error getting resource at {}: {e}",
                    local_filename.display()
                )
            })?)
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let Configuration {
        html_cache_path,
        listening_port,
    } = Configuration::load().unwrap();

    let html_cache = HTMLCache {
        directory: html_cache_path,
    };

    let app = Router::new()
        .route("/favicon.ico", get(|| async {}))
        .route("/*file", get(serve_path).with_state(html_cache));

    let listener = TcpListener::bind(("localhost", listening_port))
        .await
        .unwrap();

    info!("Bound to {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap()
}
