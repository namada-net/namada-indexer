use std::collections::BTreeSet;
use std::convert::identity;
use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDateTime, Utc};
use clap::Parser;
use deadpool_diesel::postgres::Object;
use governance::config::AppConfig;
use governance::repository;
use governance::services::{
    namada as namada_service, tendermint as tendermint_service,
};
use governance::state::AppState;
use namada_governance::storage::proposal::{AddRemove, PGFAction, PGFTarget};
use namada_sdk::time::DateTimeUtc;
use orm::migrations::CustomMigrationSource;
use shared::balance::Amount as NamadaAmount;
use shared::client::Client;
use shared::crawler;
use shared::crawler_state::{CrawlerName, IntervalCrawlerState};
use shared::error::{AsDbError, AsRpcError, ContextDbInteractError, MainError};
use shared::id::Id;
use shared::pgf::{PaymentKind, PaymentRecurrence, PgfAction, PgfPayment};
use shared::proposal::GovernanceProposalResult;
use tendermint_rpc::HttpClient;
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<(), MainError> {
    let config = AppConfig::parse();

    config.log.init();

    tracing::info!("version: {}", env!("VERGEN_GIT_SHA").to_string());

    let client = Client::new(&config.tendermint_url);

    let chain_id = tendermint_service::query_status(client.as_ref())
        .await
        .into_rpc_error()?
        .node_info
        .network
        .to_string();

    tracing::info!("Network chain id: {}", chain_id);

    let app_state = AppState::new(config.database_url).into_db_error()?;

    let conn = Arc::new(app_state.get_db_connection().await.into_db_error()?);

    // Initially set the instant to the current time minus the sleep_for
    // so we can start processing right away
    let instant = Arc::new(Mutex::new(
        Instant::now()
            .checked_sub(Duration::from_secs(config.sleep_for))
            .unwrap(),
    ));

    // Run migrations
    CustomMigrationSource::new(chain_id)
        .run_migrations(&conn)
        .await
        .expect("Should be able to run migrations");

    crawler::crawl(
        move |_| {
            crawling_fn(
                conn.clone(),
                Arc::new(client.get()),
                instant.clone(),
                config.sleep_for,
            )
        },
        0,
        None,
    )
    .await
}

async fn crawling_fn(
    conn: Arc<Object>,
    client: Arc<HttpClient>,
    instant: Arc<Mutex<Instant>>,
    sleep_for: u64,
) -> Result<(), MainError> {
    let mut instant = instant.lock().await;

    let should_process = can_process(&instant, sleep_for);

    if !should_process {
        let timestamp = Utc::now().naive_utc();
        update_crawler_timestamp(&conn, timestamp).await?;

        tracing::trace!(
            "Not enough time has passed since last crawl, skipping..."
        );

        return Err(MainError::NoAction);
    }

    tracing::debug!("Querying governance proposals...");

    let epoch = namada_service::query_last_epoch(&client)
        .await
        .into_rpc_error()?;

    tracing::debug!("Fetched epoch is {} ...", epoch);

    let running_governance_proposals = conn
        .interact(move |conn| {
            repository::governance::get_all_running_proposals(conn)
        })
        .await
        .context_db_interact_error()
        .and_then(identity)
        .into_db_error()?;

    tracing::debug!(
        "Got {} proposals to be tallied...",
        running_governance_proposals.len()
    );

    let proposals_statuses = namada_service::get_governance_proposals_updates(
        &client,
        running_governance_proposals,
        epoch as u32,
    )
    .await
    .map_err(|_| MainError::RpcError)?;
    tracing::debug!(
        "Got {} proposals statuses updates...",
        proposals_statuses.len()
    );

    let executed_proposals = conn
        .interact(move |conn| {
            repository::governance::get_all_executed_proposals(conn, epoch)
        })
        .await
        .context_db_interact_error()
        .and_then(identity)
        .into_db_error()?
        .iter()
        .map(|(proposal_id, result)| {
            (
                *proposal_id,
                match result {
                    shared::proposal::GovernanceProposalResult::Rejected => {
                        GovernanceProposalResult::ExecutedRejected
                    }
                    shared::proposal::GovernanceProposalResult::Passed => {
                        GovernanceProposalResult::ExecutedPassed
                    }
                    _ => panic!("Fetched a non-ended proposal"),
                },
            )
        })
        .collect::<Vec<_>>();

    let pgf_payments = conn
        .interact(move |conn| {
            repository::governance::get_all_pgf_executed_proposals_data(
                conn, epoch,
            )
        })
        .await
        .context_db_interact_error()
        .and_then(identity)
        .into_db_error()?
        .into_iter()
        .filter_map(|(id, data)| {
            if let Some(data) = data {
                if let Ok(fundings) =
                    serde_json::from_str::<BTreeSet<PGFAction>>(&data)
                {
                    Some((id, fundings))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .flat_map(|(id, data)| {
            data.into_iter()
                .map(|action| match action {
                    PGFAction::Retro(target) => match target {
                        PGFTarget::Internal(inner) => PgfPayment {
                            proposal_id: id,
                            recurrence: PaymentRecurrence::Retro,
                            kind: PaymentKind::Native,
                            receipient: Id::from(inner.target),
                            amount: NamadaAmount::from(inner.amount),
                            action: None,
                        },
                        PGFTarget::Ibc(inner) => PgfPayment {
                            proposal_id: id,
                            recurrence: PaymentRecurrence::Retro,
                            kind: PaymentKind::Ibc,
                            receipient: Id::Account(inner.target),
                            amount: NamadaAmount::from(inner.amount),
                            action: None,
                        },
                    },
                    PGFAction::Continuous(add_remove) => match add_remove {
                        AddRemove::Add(target) => match target {
                            PGFTarget::Internal(inner) => PgfPayment {
                                proposal_id: id,
                                recurrence: PaymentRecurrence::Continuous,
                                kind: PaymentKind::Native,
                                receipient: Id::from(inner.target),
                                amount: NamadaAmount::from(inner.amount),
                                action: Some(PgfAction::Add),
                            },
                            PGFTarget::Ibc(inner) => PgfPayment {
                                proposal_id: id,
                                recurrence: PaymentRecurrence::Continuous,
                                kind: PaymentKind::Ibc,
                                receipient: Id::Account(inner.target),
                                amount: NamadaAmount::from(inner.amount),
                                action: Some(PgfAction::Add),
                            },
                        },
                        AddRemove::Remove(target) => match target {
                            PGFTarget::Internal(inner) => PgfPayment {
                                proposal_id: id,
                                recurrence: PaymentRecurrence::Continuous,
                                kind: PaymentKind::Native,
                                receipient: Id::from(inner.target),
                                amount: NamadaAmount::from(inner.amount),
                                action: Some(PgfAction::Remove),
                            },
                            PGFTarget::Ibc(inner) => PgfPayment {
                                proposal_id: id,
                                recurrence: PaymentRecurrence::Continuous,
                                kind: PaymentKind::Ibc,
                                receipient: Id::Account(inner.target),
                                amount: NamadaAmount::from(inner.amount),
                                action: Some(PgfAction::Remove),
                            },
                        },
                    },
                })
                .collect::<Vec<PgfPayment>>()
        })
        .collect::<Vec<_>>();
    tracing::debug!("Got {} pgf payments...", pgf_payments.len());

    let timestamp = DateTimeUtc::now().0.timestamp();
    let crawler_state = IntervalCrawlerState { timestamp };

    tracing::info!(
        proposals_statuses = proposals_statuses.len(),
        pgf_payments = pgf_payments.len(),
        "Queried governance proposals successfully"
    );

    conn.interact(move |conn| {
        conn.build_transaction().read_write().run(
            |transaction_conn: &mut diesel::prelude::PgConnection| {
                for proposal_status in proposals_statuses {
                    repository::governance::update_proposal_status(
                        transaction_conn,
                        proposal_status.id,
                        proposal_status.into(),
                    )?;
                }

                repository::pgf::update_pgf(transaction_conn, pgf_payments)?;

                for (proposal_id, proposal_result) in executed_proposals {
                    repository::governance::update_proposal_result(
                        transaction_conn,
                        proposal_id,
                        proposal_result.into(),
                    )?;
                }

                repository::crawler_state::upsert_crawler_state(
                    transaction_conn,
                    (CrawlerName::Governance, crawler_state).into(),
                )?;

                anyhow::Ok(())
            },
        )
    })
    .await
    .context_db_interact_error()
    .and_then(identity)
    .into_db_error()?;

    tracing::info!(sleep_for = sleep_for, "Inserted governance into database");

    // Once we are done processing, we reset the instant
    *instant = Instant::now();

    Ok(())
}

fn can_process(instant: &MutexGuard<Instant>, sleep_for: u64) -> bool {
    let time_elapsed = instant.elapsed().as_secs();
    time_elapsed >= sleep_for
}

async fn update_crawler_timestamp(
    conn: &Object,
    timestamp: NaiveDateTime,
) -> Result<(), MainError> {
    conn.interact(move |transaction_conn| {
        repository::crawler_state::update_timestamp(
            transaction_conn,
            timestamp,
        )?;

        anyhow::Ok(())
    })
    .await
    .context_db_interact_error()
    .and_then(identity)
    .into_db_error()
}
