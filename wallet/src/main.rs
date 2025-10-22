use wallet::api::server;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger (set RUST_LOG=debug for verbose output, RUST_LOG=info for normal)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    // Read bind address from environment variable, default to 0.0.0.0:3000 for production
    // Use BIND_ADDRESS=127.0.0.1:3000 for local development
    let addr = env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    
    log::info!("Starting RGB-compatible Bitcoin wallet server on {}", addr);
    server::start_server(&addr).await?;
    Ok(())
}
