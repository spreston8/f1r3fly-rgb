use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::wallet::manager::WalletManager;
use super::handlers;

pub async fn start_server(addr: &str) -> anyhow::Result<()> {
    let wallet_manager = Arc::new(WalletManager::new());

    let app = Router::new()
        .route("/api/wallet/create", post(handlers::create_wallet_handler))
        .route("/api/wallet/import", post(handlers::import_wallet_handler))
        .route("/api/wallet/list", get(handlers::list_wallets_handler))
        .route(
            "/api/wallet/:name/addresses",
            get(handlers::get_addresses_handler),
        )
        .route(
            "/api/wallet/:name/primary-address",
            get(handlers::get_primary_address_handler),
        )
        .route(
            "/api/wallet/:name/balance",
            get(handlers::get_balance_handler),
        )
        .route("/api/wallet/:name/sync", post(handlers::sync_wallet_handler))
        .route("/api/wallet/:name/create-utxo", post(handlers::create_utxo_handler))
        .route("/api/wallet/:name/unlock-utxo", post(handlers::unlock_utxo_handler))
        .route("/api/wallet/:name/issue-asset", post(handlers::issue_asset_handler))
        .route("/api/wallet/:name/generate-invoice", post(handlers::generate_invoice_handler))
        .route("/api/wallet/:name/send-transfer", post(handlers::send_transfer_handler))
        .route("/api/wallet/:name/accept-consignment", post(handlers::accept_consignment_handler))
        .route("/api/wallet/:name/export-genesis/:contract_id", get(handlers::export_genesis_handler))
        .route("/api/consignment/:filename", get(handlers::download_consignment_handler))
        .route("/api/genesis/:filename", get(handlers::download_genesis_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(wallet_manager);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Server listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

