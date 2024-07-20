use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use axum::extract::{Path as URLPath, State};
use axum::response::{self, Html, IntoResponse, Response};
use axum::routing::get;
use axum::{http::header, Router};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::{fs, io};

use tracing::info;

mod configuration;
use configuration::Configuration;

async fn run_markdown(input_markdown: &Path, output_html: &Path) -> io::Result<ExitStatus> {
    info!("Converting {:?} to {:?}", input_markdown, output_html);

    let title = input_markdown
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    Command::new("pandoc")
        .env_clear()
        .args([
            "-f",
            "markdown",
            "-t",
            "html",
            "-s",
            &format!("--metadata=title:{}", title),
            "-o",
            output_html
                .to_str()
                .expect("Could not convert input path into UTF-8"),
            input_markdown
                .to_str()
                .expect("Could not convert output path into UTF-8"),
        ])
        .status()
        .await
}

#[derive(Clone)]
struct HTMLCache {
    directory: PathBuf,
}

impl HTMLCache {
    async fn cache_markdown(&self, input_markdown: &Path) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.directory).await?;

        let mut output_html = self.directory.clone();
        output_html.push(input_markdown.file_stem().unwrap());
        output_html.set_extension("html");

        let (output_exists, output_metadata, input_metadata) = tokio::join!(
            fs::try_exists(&output_html),
            fs::metadata(&output_html),
            fs::metadata(&input_markdown)
        );

        let should_run =
            !output_exists? || output_metadata?.modified()? < input_metadata?.modified()?;

        if should_run {
            run_markdown(input_markdown, &output_html).await?;
        } else {
            info!(
                "Using cached copy of {:?}, which is at {:?}",
                input_markdown, output_html
            );
        }

        Ok(output_html)
    }
}

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
