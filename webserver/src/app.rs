use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::http::{HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{BoxError, Json, Router};
use axum_prometheus::PrometheusMetricLayer;
use lazy_static::lazy_static;
use serde_json::json;
use shared::client::Client;
use tower::ServiceBuilder;
use tower::buffer::BufferLayer;
use tower::limit::RateLimitLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::appstate::AppState;
use crate::config::AppConfig;
use crate::handler::{
    balance as balance_handlers, block as block_handlers,
    chain as chain_handlers, crawler_state as crawler_state_handlers,
    gas as gas_handlers, governance as gov_handlers, ibc as ibc_handler,
    masp as masp_handlers, pgf as pgf_service, pk as pk_handlers,
    pos as pos_handlers, transaction as transaction_handlers,
};
use crate::state::common::CommonState;

lazy_static! {
    static ref HTTP_TIMEOUT: u64 = 60;
    static ref REQ_PER_SEC: u64 = u64::MAX;
}

pub struct ApplicationServer;

impl ApplicationServer {
    pub async fn serve(config: AppConfig) -> anyhow::Result<()> {
        let db_url = config.database_url.clone();
        let cache_url = config.cache_url.clone();

        let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

        let app_state = AppState::new(db_url, cache_url);
        let client = Client::new(&config.tendermint_url);

        let routes = {
            let common_state = CommonState::new(
                client.get(),
                config.clone(),
                app_state.clone(),
            );

            Router::new()
                .route("/pos/validator", get(pos_handlers::get_validators))
                .route(
                    "/pos/validator/all",
                    get(pos_handlers::get_all_validators),
                )
                .route("/pos/bond/:address", get(pos_handlers::get_bonds))
                .route(
                    "/pos/merged-bonds/:address",
                    get(pos_handlers::get_merged_bonds),
                )
                .route("/pos/unbond/:address", get(pos_handlers::get_unbonds))
                .route(
                    "/pos/merged-unbonds/:address",
                    get(pos_handlers::get_merged_unbonds),
                )
                .route(
                    "/pos/withdraw/:address",
                    get(pos_handlers::get_withdraws),
                )
                .route("/pos/reward/:address", get(pos_handlers::get_rewards))
                .route(
                    "/pos/reward/:delegator/:validator/:epoch",
                    get(pos_handlers::get_rewards_by_delegator_and_validator_and_epoch),
                )
                .route(
                    "/pos/voting-power",
                    get(pos_handlers::get_total_voting_power),
                )
                .route(
                    "/gov/proposal",
                    get(gov_handlers::get_governance_proposals),
                )
                .route(
                    "/gov/proposal/all",
                    get(gov_handlers::get_all_governance_proposals),
                )
                .route(
                    "/gov/proposal/:id",
                    get(gov_handlers::get_governance_proposal_by_id),
                )
                .route(
                    "/gov/proposal/:id/data",
                    get(gov_handlers::get_proposal_data_by_proposal_id),
                )
                .route(
                    "/gov/proposal/:id/votes",
                    get(gov_handlers::get_governance_proposal_votes),
                )
                .route(
                    "/gov/proposal/:id/votes/:address",
                    get(gov_handlers::get_governance_proposal_votes_by_address),
                )
                .route(
                    "/gov/voter/:address/votes",
                    get(gov_handlers::get_governance_proposal_votes_by_voter),
                )
                .route(
                    "/account/:address",
                    get(balance_handlers::get_address_balance),
                )
                .route(
                    "/revealed-public-key/:address",
                    get(pk_handlers::get_revealed_pk),
                )
                .route("/gas/estimate", get(gas_handlers::get_gas_estimate))
                .route(
                    "/gas-price/:token",
                    get(gas_handlers::get_gas_price_by_token),
                )
                .route("/gas-price", get(gas_handlers::get_all_gas_prices))
                .route(
                    "/chain/wrapper/:id",
                    get(transaction_handlers::get_wrapper_tx),
                )
                .route(
                    "/chain/inner/:id",
                    get(transaction_handlers::get_inner_tx),
                )
                .route(
                    "/chain/history",
                    get(transaction_handlers::get_transaction_history),
                )
                .route("/chain/parameters", get(chain_handlers::get_parameters))
                .route("/chain/rpc-url", get(chain_handlers::get_rpc_url))
                .route("/chain/token", get(chain_handlers::get_tokens))
                .route(
                    "/chain/token-supply",
                    get(chain_handlers::get_token_supply),
                )
                .route(
                    "/chain/circulating-supply",
                    get(chain_handlers::get_circulating_supply),
                )
                .route(
                    "/chain/block/latest",
                    get(chain_handlers::get_last_processed_block),
                )
                .route(
                    "/chain/epoch/latest",
                    get(chain_handlers::get_last_processed_epoch),
                )
                .route("/ibc/:tx_id/status", get(ibc_handler::get_ibc_status))
                .route(
                    "/ibc/rate-limits",
                    get(ibc_handler::get_ibc_rate_limits),
                )
                .route(
                    "/ibc/token-flows",
                    get(ibc_handler::get_ibc_token_flows),
                )
                .route(
                    "/ibc/token-throughput/:token",
                    get(ibc_handler::get_ibc_token_throughput),
                )
                .route(
                    "/pgf/payments",
                    get(pgf_service::get_pgf_continuous_payments),
                )
                .route(
                    "/pgf/paymenents/:proposal_id",
                    get(pgf_service::get_pgf_payment_by_proposal_id),
                )
                .route(
                    "/crawlers/timestamps",
                    get(crawler_state_handlers::get_crawlers_timestamps),
                )
                // Server sent events endpoints
                .route("/chain/status", get(chain_handlers::chain_status))
                .route(
                    "/block/height/:value",
                    get(block_handlers::get_block_by_height),
                )
                .route(
                    "/block/timestamp/:value",
                    get(block_handlers::get_block_by_timestamp),
                )
                .route(
                    "/block/hash/:value",
                    get(block_handlers::get_block_by_hash),
                )
                .route(
                    "/masp/aggregates",
                    get(masp_handlers::get_masp_aggregates),
                )
                .route(
                    "/metrics",
                    get(|| async move { metric_handle.render() }),
                )
                .with_state(common_state)
        };

        let cors = CorsLayer::new()
            .allow_origin("*".parse::<HeaderValue>().unwrap())
            .allow_methods(Any)
            .allow_headers(Any);

        let router = Router::new()
            .nest("/api/v1", routes)
            .merge(Router::new().route(
                "/health",
                get(|| async { json!({"commit": env!("VERGEN_GIT_SHA").to_string(), "version": env!("CARGO_PKG_VERSION") }).to_string() }),
            ))
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(prometheus_layer)
                    .layer(HandleErrorLayer::new(Self::handle_timeout_error))
                    .timeout(Duration::from_secs(*HTTP_TIMEOUT))
                    .layer(cors)
                    .layer(BufferLayer::new(4096))
                    .layer(RateLimitLayer::new(
                        *REQ_PER_SEC,
                        Duration::from_secs(1),
                    )),
            );

        let router = router.fallback(Self::handle_404);

        let port = config.port;
        let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));

        tracing::info!("🚀 Server has launched on https://{addr}");

        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

        axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(Self::shutdown_signal())
            .await
            .unwrap_or_else(|e| panic!("Server error: {}", e));

        Ok(())
    }

    /// Adds a custom handler for tower's `TimeoutLayer`, see https://docs.rs/axum/latest/axum/middleware/index.html#commonly-used-middleware.
    async fn handle_timeout_error(
        err: BoxError,
    ) -> (StatusCode, Json<serde_json::Value>) {
        if err.is::<tower::timeout::error::Elapsed>() {
            (
                StatusCode::REQUEST_TIMEOUT,
                Json(json!({
                    "error":
                        format!(
                            "request took longer than the configured {} second timeout",
                            *HTTP_TIMEOUT
                        )
                })),
            )
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("unhandled internal error: {}", err)
                })),
            )
        }
    }

    /// Tokio signal handler that will wait for a user to press CTRL+C.
    /// We use this in our hyper `Server` method `with_graceful_shutdown`.
    async fn shutdown_signal() {
        tokio::signal::ctrl_c()
            .await
            .expect("expect tokio signal ctrl-c");
        tracing::warn!("signal shutdown");
    }

    async fn handle_404() -> impl IntoResponse {
        (
            StatusCode::NOT_FOUND,
            axum::response::Json(serde_json::json!({
                    "errors":{
                    "message": vec!(String::from("The requested resource does not exist on this server!")),}
                }
            )),
        )
    }
}
