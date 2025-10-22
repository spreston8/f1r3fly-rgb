use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use super::handlers;
use crate::wallet::manager::WalletManager;

pub async fn start_server(addr: &str) -> anyhow::Result<()> {
    let wallet_manager = Arc::new(WalletManager::new());

    // Start RGB runtime lifecycle manager (Phase 1)
    wallet_manager.start_lifecycle_manager();

    let app = Router::new()
        // Firefly integration
        .route(
            "/api/firefly/status",
            get(handlers::get_firefly_status_handler),
        )
        // Wallet routes
        .route("/api/wallet/create", post(handlers::create_wallet_handler))
        .route("/api/wallet/import", post(handlers::import_wallet_handler))
        .route("/api/wallet/list", get(handlers::list_wallets_handler))
        .route("/api/wallet/:name", delete(handlers::delete_wallet_handler))
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
        .route(
            "/api/wallet/:name/sync",
            post(handlers::sync_wallet_handler),
        )
        .route(
            "/api/wallet/:name/sync-rgb",
            post(handlers::sync_rgb_handler),
        )
        .route(
            "/api/wallet/:name/create-utxo",
            post(handlers::create_utxo_handler),
        )
        .route(
            "/api/wallet/:name/unlock-utxo",
            post(handlers::unlock_utxo_handler),
        )
        .route(
            "/api/wallet/:name/send-bitcoin",
            post(handlers::send_bitcoin_handler),
        )
        .route(
            "/api/wallet/:name/issue-asset",
            post(handlers::issue_asset_handler),
        )
        .route(
            "/api/wallet/:name/issue-asset-firefly",
            post(handlers::issue_asset_with_firefly_handler),
        )
        .route(
            "/api/wallet/:name/generate-invoice",
            post(handlers::generate_invoice_handler),
        )
        .route(
            "/api/wallet/:name/send-transfer",
            post(handlers::send_transfer_handler),
        )
        .route(
            "/api/wallet/:name/accept-consignment",
            post(handlers::accept_consignment_handler),
        )
        .route(
            "/api/wallet/:name/export-genesis/:contract_id",
            get(handlers::export_genesis_handler),
        )
        .route(
            "/api/consignment/:filename",
            get(handlers::download_consignment_handler),
        )
        .route(
            "/api/genesis/:filename",
            get(handlers::download_genesis_handler),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(wallet_manager.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Server listening on http://{}", addr);

    // Serve with graceful shutdown (Phase 1)
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(wallet_manager))
        .await?;

    Ok(())
}

/// Handle graceful shutdown signals (Ctrl+C, SIGTERM)
async fn shutdown_signal(manager: Arc<WalletManager>) {
    // Wait for SIGTERM or Ctrl+C
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            log::info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            log::info!("Received SIGTERM signal");
        },
    }

    log::info!("Shutdown signal received, saving RGB runtimes...");
    if let Err(e) = manager.shutdown().await {
        log::error!("Error during shutdown: {}", e);
    }
}
