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

    // Configure CORS based on environment
    // Set ALLOWED_ORIGINS="https://your-app.vercel.app,https://your-app-preview.vercel.app" for production
    // If not set, allows any origin (development mode)
    let cors = match std::env::var("ALLOWED_ORIGINS") {
        Ok(origins) if !origins.is_empty() => {
            log::info!("CORS configured for origins: {}", origins);
            let origin_list: Vec<_> = origins
                .split(',')
                .map(|s| s.trim().parse().expect("Invalid CORS origin"))
                .collect();
            CorsLayer::new()
                .allow_origin(origin_list)
                .allow_methods(Any)
                .allow_headers(Any)
        }
        _ => {
            log::warn!("CORS: Allowing all origins (development mode). Set ALLOWED_ORIGINS env var for production.");
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        }
    };

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
        .layer(cors)
        .with_state(wallet_manager);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Server listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
