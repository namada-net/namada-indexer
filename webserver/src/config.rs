use shared::log_config::LogConfig;

#[derive(clap::Parser, Clone)]
pub struct AppConfig {
    #[clap(long, env, default_value = "5001")]
    pub port: u16,

    #[clap(long, env)]
    pub cache_url: Option<String>,

    #[clap(long, env)]
    pub database_url: String,

    #[clap(long, env)]
    pub rps: Option<u64>,

    #[clap(long, env)]
    pub tendermint_url: String,

    #[clap(flatten)]
    pub log: LogConfig,
}
