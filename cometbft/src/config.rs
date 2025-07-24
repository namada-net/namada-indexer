use core::fmt;
use std::fmt::Display;

use shared::log_config::LogConfig;

#[derive(clap::ValueEnum, Clone, Debug, Copy)]
pub enum CargoEnv {
    Development,
    Production,
}

impl Display for CargoEnv {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(clap::Parser)]
pub struct AppConfig {
    #[clap(long, env)]
    pub tendermint_url: String,

    #[clap(long, env)]
    pub database_url: String,

    #[clap(long, env, default_value_t = 100)]
    pub batch_size: usize,

    #[clap(flatten)]
    pub log: LogConfig,
}
