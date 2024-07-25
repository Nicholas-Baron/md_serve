use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitStatus,
};

use tokio::{fs, io};
use tracing::info;

async fn run_markdown(input_markdown: &Path, output_html: &Path) -> io::Result<ExitStatus> {
    use tokio::process::Command;

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
            &format!("--metadata=title:{title}"),
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
pub(crate) struct HTMLCache {
    pub directory: PathBuf,
}

impl HTMLCache {
    pub async fn cache_markdown(&self, input_markdown: &Path) -> io::Result<PathBuf> {
        if !fs::try_exists(input_markdown).await? {
            return Err(io::Error::new(io::ErrorKind::NotFound, "No such file"));
        }

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
