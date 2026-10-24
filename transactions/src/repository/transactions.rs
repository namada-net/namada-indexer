use std::collections::HashSet;

use anyhow::Context;
use chrono::NaiveDateTime;
use diesel::upsert::excluded;
use diesel::{
    ExpressionMethods, OptionalEmptyChangesetExtension, PgConnection,
    RunQueryDsl,
};
use orm::crawler_state::{BlockStateInsertDb, CrawlerNameDb};
use orm::gas::GasEstimationInsertDb;
use orm::ibc::{
    IbcAckInsertDb, IbcAckStatusDb, IbcSequencekStatusUpdateDb,
    IbcTokenFlowsInsertDb,
};
use orm::schema::{
    crawler_state, gas_estimations, ibc_ack, ibc_token_flows,
    inner_transactions, transaction_history, wrapper_transactions,
};
use orm::transactions::{
    InnerTransactionInsertDb, TransactionHistoryInsertDb,
    WrapperTransactionInsertDb,
};
use shared::crawler_state::{BlockCrawlerState, CrawlerName};
use shared::gas::GasEstimation;
use shared::transaction::{
    IbcAck, IbcSequence, IbcTokenFlow, InnerTransaction, TransactionTarget,
    WrapperTransaction,
};

pub fn insert_inner_transactions(
    transaction_conn: &mut PgConnection,
    txs: Vec<InnerTransaction>,
) -> anyhow::Result<()> {
    diesel::insert_into(inner_transactions::table)
        .values::<&Vec<InnerTransactionInsertDb>>(
            &txs.into_iter()
                .map(InnerTransactionInsertDb::from)
                .collect::<Vec<_>>(),
        )
        .on_conflict(inner_transactions::id)
        .do_update()
        .set((
            // Allow updating transactions kind + data so that if the indexer
            // is updated with new transaction type support, we can
            // easily go back & reindex any old transactions
            // that were previously marked as "unknown".
            inner_transactions::kind.eq(excluded(inner_transactions::kind)),
            inner_transactions::data.eq(excluded(inner_transactions::data)),
        ))
        .execute(transaction_conn)
        .context("Failed to insert inner transactions in db")?;

    anyhow::Ok(())
}

pub fn insert_wrapper_transactions(
    transaction_conn: &mut PgConnection,
    txs: Vec<WrapperTransaction>,
) -> anyhow::Result<()> {
    diesel::insert_into(wrapper_transactions::table)
        .values::<&Vec<WrapperTransactionInsertDb>>(
            &txs.into_iter()
                .map(WrapperTransactionInsertDb::from)
                .collect::<Vec<_>>(),
        )
        .on_conflict_do_nothing()
        .execute(transaction_conn)
        .context("Failed to insert wrapper transactions in db")?;

    anyhow::Ok(())
}

pub fn insert_crawler_state(
    transaction_conn: &mut PgConnection,
    crawler_state: BlockCrawlerState,
) -> anyhow::Result<()> {
    diesel::insert_into(crawler_state::table)
        .values::<&BlockStateInsertDb>(
            &(CrawlerName::Transactions, crawler_state).into(),
        )
        .on_conflict(crawler_state::name)
        .do_update()
        .set((
            crawler_state::timestamp.eq(excluded(crawler_state::timestamp)),
            crawler_state::last_processed_block
                .eq(excluded(crawler_state::last_processed_block)),
        ))
        .execute(transaction_conn)
        .context("Failed to update crawler state in db")?;

    anyhow::Ok(())
}

pub fn update_crawler_timestamp(
    transaction_conn: &mut PgConnection,
    timestamp: NaiveDateTime,
) -> anyhow::Result<()> {
    diesel::update(crawler_state::table)
        .filter(
            crawler_state::name
                .eq(CrawlerNameDb::from(CrawlerName::Transactions)),
        )
        .set(crawler_state::timestamp.eq(timestamp))
        .execute(transaction_conn)
        .context("Failed to update crawler timestamp in db")?;

    anyhow::Ok(())
}

pub fn insert_ibc_sequence(
    transaction_conn: &mut PgConnection,
    ibc_sequences: Vec<IbcSequence>,
) -> anyhow::Result<()> {
    diesel::insert_into(ibc_ack::table)
        .values::<Vec<IbcAckInsertDb>>(
            ibc_sequences
                .into_iter()
                .map(IbcAckInsertDb::from)
                .collect(),
        )
        .on_conflict_do_nothing()
        .execute(transaction_conn)
        .context("Failed to insert ibc sequence in db")?;

    anyhow::Ok(())
}

pub fn update_ibc_sequence(
    transaction_conn: &mut PgConnection,
    ibc_acks: Vec<IbcAck>,
) -> anyhow::Result<()> {
    for ack in ibc_acks {
        let ack_update = IbcSequencekStatusUpdateDb {
            status: IbcAckStatusDb::from(ack.status.clone()),
        };
        diesel::update(ibc_ack::table)
            .set(ack_update)
            .filter(ibc_ack::dsl::id.eq(ack.id()))
            .execute(transaction_conn)
            .optional_empty_changeset()
            .context("Failed to update validator metadata in db")?;
    }
    anyhow::Ok(())
}

pub fn insert_transactions_history(
    transaction_conn: &mut PgConnection,
    txs: HashSet<TransactionTarget>,
) -> anyhow::Result<()> {
    diesel::insert_into(transaction_history::table)
        .values::<&Vec<TransactionHistoryInsertDb>>(
            &txs.into_iter()
                .map(TransactionHistoryInsertDb::from)
                .collect::<Vec<_>>(),
        )
        .on_conflict_do_nothing()
        .execute(transaction_conn)
        .context("Failed to insert transaction history in db")?;

    anyhow::Ok(())
}

pub fn insert_gas_estimates(
    transaction_conn: &mut PgConnection,
    gas_estimates: Vec<GasEstimation>,
) -> anyhow::Result<()> {
    diesel::insert_into(gas_estimations::table)
        .values::<Vec<GasEstimationInsertDb>>(
            gas_estimates
                .into_iter()
                .map(GasEstimationInsertDb::from)
                .collect(),
        )
        .on_conflict_do_nothing()
        .execute(transaction_conn)
        .context("Failed to update gas estimates in db")?;

    anyhow::Ok(())
}

pub fn upsert_ibc_token_flows<I>(
    transaction_conn: &mut PgConnection,
    flows: I,
) -> anyhow::Result<()>
where
    I: IntoIterator<Item = IbcTokenFlow>,
{
    let flows: Vec<_> = flows
        .into_iter()
        .map(
            |IbcTokenFlow {
                 address,
                 epoch,
                 deposit,
                 withdraw,
             }| {
                IbcTokenFlowsInsertDb {
                    address,
                    deposit,
                    withdraw,
                    epoch: epoch as _,
                }
            },
        )
        .collect();

    diesel::insert_into(ibc_token_flows::table)
        .values(&flows)
        .on_conflict((
            ibc_token_flows::dsl::epoch,
            ibc_token_flows::dsl::address,
        ))
        .do_update()
        .set((
            ibc_token_flows::dsl::deposit.eq(ibc_token_flows::dsl::deposit
                + excluded(ibc_token_flows::dsl::deposit)),
            ibc_token_flows::dsl::withdraw.eq(ibc_token_flows::dsl::withdraw
                + excluded(ibc_token_flows::dsl::withdraw)),
        ))
        .execute(transaction_conn)
        .context("Failed to upsert ibc token flows in db")?;

    anyhow::Ok(())
}
