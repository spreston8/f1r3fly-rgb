/// Axum HTTP server setup and routing

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::handlers::*;
use crate::rpc_client::BitcoinRpcClient;

pub fn create_router(rpc_client: Arc<BitcoinRpcClient>) -> Router {
    // Configure CORS to allow requests from wallet frontend/tests
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(health_check))
        
        // Block endpoints
        .route("/blocks/tip/height", get(get_tip_height))
        .route("/blocks/tip/hash", get(get_tip_hash))
        
        // Transaction endpoints
        .route("/tx", post(broadcast_transaction))
        .route("/tx/:txid", get(get_transaction))
        .route("/tx/:txid/status", get(get_transaction_status))
        .route("/tx/:txid/raw", get(get_transaction_raw))
        .route("/tx/:txid/outspend/:index", get(get_output_spend_status))
        
        // Address endpoints
        .route("/address/:address/utxo", get(get_address_utxos))
        
        // Regtest helper endpoints
        .route("/regtest/mine", post(mine_blocks))
        
        // Shared state
        .with_state(rpc_client)
        
        // Middleware
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

pub async fn run_server(
    rpc_client: Arc<BitcoinRpcClient>,
    host: String,
    port: u16,
) -> anyhow::Result<()> {
    let app = create_router(rpc_client);
    
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    log::info!("🚀 Esplora mock server listening on http://{}", addr);
    log::info!("📡 Connected to Bitcoin Core RPC");
    log::info!("🔨 Regtest mining endpoint: POST /regtest/mine");
    
    axum::serve(listener, app)
        .await?;
    
    Ok(())
}

