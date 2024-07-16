use std::path::PathBuf;

use config::Config;

pub(crate) struct Configuration {
    pub html_cache_path: PathBuf,
    pub listening_port: u16,
}

impl Configuration {
    pub(crate) fn load() -> Result<Self, config::ConfigError> {
        let config = Config::builder()
            .add_source(config::File::with_name("md_serve").required(false))
            .set_default("html_cache_path", "./html_cache")
            .unwrap()
            .set_default("listening_port", 3000)
            .unwrap()
            .build()?;

        Ok(Self {
            html_cache_path: config.get("html_cache_path")?,
            listening_port: config.get("listening_port")?,
        })
    }
}
