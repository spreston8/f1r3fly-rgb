use wallet::api::server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger (set RUST_LOG=debug for verbose output, RUST_LOG=info for normal)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let addr = "127.0.0.1:3000";
    log::info!("Starting RGB-compatible Bitcoin wallet server on {}", addr);
    server::start_server(addr).await?;
    Ok(())
}
