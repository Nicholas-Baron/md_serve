use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use axum::extract::{Path as URLPath, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::{fs, io};

use tracing::info;

use config::Config;

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

        let should_run = !fs::try_exists(&output_html).await?
            || fs::metadata(&output_html).await?.modified()?
                < fs::metadata(&input_markdown).await?.modified()?;

        if should_run {
            run_markdown(input_markdown, &output_html).await?;
        } else {
            info!(
                "Using cached copy of {:?}, which is at {:?}",
                input_markdown, output_html
            );
        }

        if fs::try_exists(&output_html).await? {
            Ok(output_html)
        } else {
            todo!()
        }
    }
}

async fn serve_html(
    URLPath(path): URLPath<String>,
    State(html_cache): State<HTMLCache>,
) -> Html<String> {
    let mut input_markdown = PathBuf::from(path.clone());
    input_markdown.set_extension("md");
    info!(
        "{path} requested. Sourcing from {}",
        input_markdown.display()
    );

    let output_html = html_cache.cache_markdown(&input_markdown).await.unwrap();
    Html(fs::read_to_string(output_html).await.unwrap())
}

struct Configuration {
    html_cache_path: PathBuf,
}

impl Configuration {
    fn load() -> Result<Self, config::ConfigError> {
        let config = Config::builder()
            .add_source(config::File::with_name("md_serve").required(false))
            .set_default("html_cache_path", "./html_cache")
            .unwrap()
            .build()?;

        Ok(Self {
            html_cache_path: config.get_string("html_cache_path")?.into(),
        })
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let Configuration { html_cache_path } = Configuration::load().unwrap();

    let html_cache = HTMLCache {
        directory: PathBuf::from(html_cache_path),
    };

    let app = Router::new()
        .route("/favicon.ico", get(|| async {}))
        .route("/*file", get(serve_html).with_state(html_cache));

    let listener = TcpListener::bind("localhost:3000").await.unwrap();

    info!("Bound to {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap()
}
