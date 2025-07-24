use tendermint_rpc::endpoint::block::Response as TendermintBlockResponse;
use tendermint_rpc::endpoint::block_results::Response as TendermintBlockResultResponse;

#[derive(Debug, Clone)]
pub struct CometbftBlock {
    pub block_height: u32,
    pub block: TendermintBlockResponse,
    pub events: TendermintBlockResultResponse,
    pub epoch: u32,
}
