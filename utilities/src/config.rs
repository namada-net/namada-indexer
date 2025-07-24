#[derive(clap::Parser)]
pub struct AppConfig {
    #[clap(long, env)]
    pub tendermint_url: String,

    #[clap(long, env)]
    pub fix_tx: bool,

    #[clap(long, env)]
    pub deserialize_tx: bool,

    #[clap(long, env)]
    pub query_account: bool,

    #[clap(long, env)]
    pub block_height: Option<u32>,

    #[clap(long, env)]
    pub address: Option<String>,
}
