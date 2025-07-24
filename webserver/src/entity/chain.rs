use bigdecimal::ToPrimitive;
use orm::parameters::ParametersDb;
use serde_json::Value as SerdeJSONValue;
use shared::id::Id;

#[derive(Clone, Debug)]
pub struct Parameters {
    pub unbonding_length: u64,
    pub pipeline_length: u64,
    pub epochs_per_year: u64,
    pub apr: f64,
    pub native_token_address: Id,
    pub chain_id: String,
    pub genesis_time: String,
    pub min_duration: String,
    pub min_num_of_blocks: String,
    pub max_block_time: String,
    pub checksums: SerdeJSONValue,
    pub epoch_switch_blocks_delay: u64,
    pub cubic_slashing_window_length: u64,
    pub duplicate_vote_min_slash_rate: u64,
    pub light_client_attack_min_slash_rate: u64,
}

impl From<ParametersDb> for Parameters {
    fn from(parameters: ParametersDb) -> Self {
        Self {
            unbonding_length: parameters.unbonding_length as u64,
            pipeline_length: parameters.pipeline_length as u64,
            epochs_per_year: parameters.epochs_per_year as u64,
            apr: parameters.apr.parse().expect("Should be a valid float"),
            native_token_address: Id::Account(parameters.native_token_address),
            chain_id: parameters.chain_id,
            genesis_time: parameters.genesis_time.to_string(),
            min_duration: parameters.min_duration.to_string(),
            min_num_of_blocks: parameters.min_num_of_blocks.to_string(),
            max_block_time: parameters.max_block_time.to_string(),
            checksums: parameters.checksums,
            epoch_switch_blocks_delay: parameters.epoch_switch_blocks_delay
                as u64,
            cubic_slashing_window_length: parameters
                .cubic_slashing_window_length
                as u64,
            duplicate_vote_min_slash_rate: parameters
                .duplicate_vote_min_slash_rate
                .to_u64()
                .expect("Should be a valid u64"),
            light_client_attack_min_slash_rate: parameters
                .light_client_attack_min_slash_rate
                .to_u64()
                .expect("Should be a valid u64"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenSupply {
    pub address: Id,
    pub total_supply: u64,
    pub effective_supply: Option<u64>,
}
