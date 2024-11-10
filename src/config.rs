use std::path::PathBuf;

use clap::Parser;
use eyre::Context;

pub fn load_cfg() -> eyre::Result<Config> {
    let cfg = CliConfig::try_parse().wrap_err("failed to parse command line config")?;

    Ok(cfg.into())
}

#[derive(Debug, Clone)]
pub struct Config {
    pub url: String,
    pub dir: PathBuf,
}

impl Config {
    pub fn new(url: String, dir: PathBuf) -> Self {
        Self { url, dir }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new("127.0.0.1:9812".to_string(), ".".into())
    }
}

impl From<CliConfig> for Config {
    fn from(value: CliConfig) -> Self {
        let default = Config::default();

        let dir = value.dir;
        let url = value.url.unwrap_or(default.url);

        Self::new(url, dir)
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CliConfig {
    #[arg(short, long)]
    url: Option<String>,

    #[arg(short, long, value_name = "ASSET_DIR")]
    dir: PathBuf,
}
