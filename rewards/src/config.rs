use shared::log_config::LogConfig;

#[derive(clap::Parser)]
pub struct AppConfig {
    #[clap(long, env)]
    pub tendermint_url: String,

    #[clap(long, env, default_value_t = 60)]
    pub sleep_for: u64,

    #[clap(
        long,
        help = "Crawl from given epoch and do not update crawler_state"
    )]
    pub backfill_from: Option<u32>,

    #[clap(long, env)]
    pub database_url: String,

    #[clap(flatten)]
    pub log: LogConfig,
}
