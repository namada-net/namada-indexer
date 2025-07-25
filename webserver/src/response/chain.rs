use serde::{Deserialize, Serialize};
use serde_json::Value as SerdeJSONValue;
use shared::token::Token as SharedToken;

use crate::entity::chain::{CirculatingSupply, Parameters, TokenSupply};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParametersResponse {
    pub unbonding_length: u64,
    pub pipeline_length: u64,
    pub epochs_per_year: u64,
    pub apr: f64,
    pub native_token_address: String,
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

impl From<Parameters> for ParametersResponse {
    fn from(parameters: Parameters) -> Self {
        Self {
            unbonding_length: parameters.unbonding_length,
            pipeline_length: parameters.pipeline_length,
            epochs_per_year: parameters.epochs_per_year,
            apr: parameters.apr,
            native_token_address: parameters.native_token_address.to_string(),
            chain_id: parameters.chain_id,
            genesis_time: parameters.genesis_time.to_string(),
            min_duration: parameters.min_duration.to_string(),
            min_num_of_blocks: parameters.min_num_of_blocks.to_string(),
            max_block_time: parameters.max_block_time.to_string(),
            checksums: parameters.checksums,
            epoch_switch_blocks_delay: parameters.epoch_switch_blocks_delay,
            cubic_slashing_window_length: parameters
                .cubic_slashing_window_length,
            duplicate_vote_min_slash_rate: parameters
                .duplicate_vote_min_slash_rate,
            light_client_attack_min_slash_rate: parameters
                .light_client_attack_min_slash_rate,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcUrlResponse {
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastProcessedBlockResponse {
    pub block: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastProcessedEpochResponse {
    pub epoch: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeToken {
    pub address: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IbcToken {
    pub address: String,
    pub trace: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum TokenResponse {
    Native(NativeToken),
    Ibc(IbcToken),
}

impl From<SharedToken> for TokenResponse {
    fn from(value: SharedToken) -> Self {
        match value {
            SharedToken::Native(token) => TokenResponse::Native(NativeToken {
                address: token.to_string(),
            }),
            SharedToken::Ibc(token) => TokenResponse::Ibc(IbcToken {
                address: token.address.to_string(),
                trace: token.trace.unwrap_or_default().to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenSupplyResponse {
    pub address: String,
    pub total_supply: u64,
    pub effective_supply: Option<u64>,
}

impl From<TokenSupply> for TokenSupplyResponse {
    fn from(value: TokenSupply) -> Self {
        Self {
            address: value.address.to_string(),
            total_supply: value.total_supply,
            effective_supply: value.effective_supply,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CirculatingSupplyResponse {
    pub circulating_supply: String,
}

impl From<CirculatingSupply> for CirculatingSupplyResponse {
    fn from(value: CirculatingSupply) -> Self {
        Self {
            circulating_supply: value.circulating_supply,
        }
    }
}
