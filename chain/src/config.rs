use shared::log_config::LogConfig;

#[derive(clap::Parser)]
pub struct AppConfig {
    #[clap(long, env)]
    pub tendermint_url: String,

    #[clap(long, env)]
    pub database_url: String,

    #[clap(
        long,
        env,
        default_value = "100",
        help = "Time between retry attempts in milliseconds"
    )]
    pub initial_query_retry_time: u64,

    #[clap(long, env, default_value = "5")]
    pub initial_query_retry_attempts: usize,

    #[clap(
        long,
        help = "Crawl from given height and do not update crawler_state"
    )]
    pub backfill_from: Option<u32>,

    #[clap(
        long,
        env,
        default_value = "3600",
        help = "Crawl from given height and do not update crawler_state"
    )]
    pub storage_read_past_height_limit: u32,

    #[clap(flatten)]
    pub log: LogConfig,

    #[clap(
        short,
        long,
        env,
        help = "Clear (un)bonds from DB and re-query them.",
        default_value = "false"
    )]
    pub reindex_bonds: bool,
}
