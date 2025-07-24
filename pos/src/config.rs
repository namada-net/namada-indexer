use shared::log_config::LogConfig;

#[derive(clap::Parser)]
pub struct AppConfig {
    #[clap(long, env)]
    pub tendermint_url: String,

    #[clap(long, env)]
    pub database_url: String,

    #[clap(flatten)]
    pub log: LogConfig,
}
