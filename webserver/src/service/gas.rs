use bigdecimal::ToPrimitive;

use crate::appstate::AppState;
use crate::entity::gas::{GasEstimate, GasPrice};
use crate::entity::transaction::TransactionKind;
use crate::error::gas::GasError;
use crate::repository::chain::{ChainRepository, ChainRepositoryTrait};
use crate::repository::gas::{GasRepository, GasRepositoryTrait};

#[derive(Clone)]
pub struct GasService {
    gas_repo: GasRepository,
    chain_repo: ChainRepository,
    default_gas_table: DefaultGasTable,
}

impl GasService {
    pub fn new(app_state: AppState) -> Self {
        Self {
            gas_repo: GasRepository::new(app_state.clone()),
            chain_repo: ChainRepository::new(app_state),
            default_gas_table: DefaultGasTable::default(),
        }
    }

    pub async fn get_gas_price_by_token(
        &self,
        token: String,
    ) -> Result<Vec<GasPrice>, GasError> {
        let tokens = self
            .chain_repo
            .find_tokens()
            .await
            .map_err(GasError::Database)?;

        self.gas_repo
            .find_gas_price_by_token(token)
            .await
            .map_err(GasError::Database)
            .map(|r| {
                r.iter()
                    .cloned()
                    .map(|gas_price| {
                        GasPrice::from_db(gas_price, tokens.clone())
                    })
                    .collect()
            })
    }

    pub async fn get_all_gas_prices(&self) -> Result<Vec<GasPrice>, GasError> {
        let tokens = self
            .chain_repo
            .find_tokens()
            .await
            .map_err(GasError::Database)?;

        self.gas_repo
            .find_all_gas_prices()
            .await
            .map_err(GasError::Database)
            .map(|r| {
                r.iter()
                    .cloned()
                    .map(|gas_price| {
                        GasPrice::from_db(gas_price, tokens.clone())
                    })
                    .collect()
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn estimate_gas(
        &self,
        bond: u64,
        redelegate: u64,
        claim_rewards: u64,
        unbond: u64,
        transparent_transfer: u64,
        shielded_transfer: u64,
        shielding_transfer: u64,
        unshielding_transfer: u64,
        vote: u64,
        ibc_shielding_transfer: u64,
        ibc_unshielding_transfer: u64,
        ibc_transparent_transfer: u64,
        withdraw: u64,
        reveal_pk: u64,
        signatures: u64,
        tx_size: u64,
    ) -> Result<GasEstimate, GasError> {
        let (min, max, avg, count) = self
            .gas_repo
            .find_gas_estimates(
                bond,
                redelegate,
                claim_rewards,
                unbond,
                transparent_transfer,
                shielded_transfer,
                shielding_transfer,
                unshielding_transfer,
                vote,
                ibc_shielding_transfer,
                ibc_unshielding_transfer,
                ibc_transparent_transfer,
                withdraw,
                reveal_pk,
                signatures,
                tx_size,
            )
            .await
            .map_err(GasError::Database)
            .map(|(min, max, avg, count)| {
                let min = min.map(|gas| gas as u64);
                let max = max.map(|gas| gas as u64);
                let avg = avg.map(|gas| gas.to_f64().unwrap() as u64);
                let count = count as u64;
                (min, max, avg, count)
            })?;

        if let (Some(min), Some(max), Some(avg), count) = (min, max, avg, count)
        {
            Ok(GasEstimate {
                min,
                max,
                avg,
                total_estimates: count,
            })
        } else {
            let mut estimate = 0;
            estimate += bond
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::Bond);
            estimate += claim_rewards
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::ClaimRewards);
            estimate += unbond
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::Unbond);
            estimate += transparent_transfer
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::TransparentTransfer);
            estimate += shielded_transfer
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::ShieldedTransfer);
            estimate += shielding_transfer
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::ShieldingTransfer);
            estimate += unshielding_transfer
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::UnshieldingTransfer);
            estimate += vote
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::VoteProposal);
            estimate += ibc_shielding_transfer
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::IbcShieldingTransfer);
            estimate += ibc_unshielding_transfer
                * self.default_gas_table.get_gas_by_tx_kind(
                    TransactionKind::IbcUnshieldingTransfer,
                );
            estimate += withdraw
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::Withdraw);
            estimate += reveal_pk
                * self
                    .default_gas_table
                    .get_gas_by_tx_kind(TransactionKind::RevealPk);

            Ok(GasEstimate {
                min: estimate,
                max: estimate,
                avg: estimate,
                total_estimates: 0,
            })
        }
    }
}

#[derive(Debug, Clone, Default)]
struct DefaultGasTable {}

impl DefaultGasTable {
    fn get_gas_by_tx_kind(&self, tx_kind: TransactionKind) -> u64 {
        match tx_kind {
            TransactionKind::Bond => 50000,
            TransactionKind::ClaimRewards => 50000,
            TransactionKind::Unbond => 50000,
            TransactionKind::TransparentTransfer => 50000,
            TransactionKind::ShieldedTransfer => 50000,
            TransactionKind::ShieldingTransfer => 50000,
            TransactionKind::UnshieldingTransfer => 50000,
            TransactionKind::VoteProposal => 50000,
            TransactionKind::IbcShieldingTransfer => 50000,
            TransactionKind::IbcUnshieldingTransfer => 50000,
            TransactionKind::Withdraw => 50000,
            TransactionKind::RevealPk => 25000,
            TransactionKind::MixedTransfer => 50000,
            TransactionKind::Redelegation => 100000,
            TransactionKind::InitProposal => 50000,
            TransactionKind::IbcMsgTransfer => 50000,
            TransactionKind::IbcTransparentTransfer => 50000,
            _ => 0,
        }
    }
}
