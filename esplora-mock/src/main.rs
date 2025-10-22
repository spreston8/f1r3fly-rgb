/// Esplora Mock Server
/// 
/// A lightweight mock server that translates Esplora API calls to Bitcoin Core RPC.
/// Designed for Regtest testing and development.

mod handlers;
mod rpc_client;
mod server;
mod types;

use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;

use rpc_client::BitcoinRpcClient;
use server::run_server;

#[derive(Debug)]
struct Config {
    // Bitcoin Core RPC
    bitcoin_rpc_url: String,
    bitcoin_rpc_user: String,
    bitcoin_rpc_password: String,
    
    // Server
    server_host: String,
    server_port: u16,
}

impl Config {
    fn from_env() -> Result<Self> {
        dotenv::dotenv().ok(); // Load .env file if present
        
        let bitcoin_rpc_url = env::var("BITCOIN_RPC_URL")
            .unwrap_or_else(|_| "http://localhost:18443".to_string());
        
        let bitcoin_rpc_user = env::var("BITCOIN_RPC_USER")
            .context("BITCOIN_RPC_USER environment variable not set")?;
        
        let bitcoin_rpc_password = env::var("BITCOIN_RPC_PASSWORD")
            .context("BITCOIN_RPC_PASSWORD environment variable not set")?;
        
        let server_host = env::var("SERVER_HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());
        
        let server_port = env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .context("Invalid SERVER_PORT")?;
        
        Ok(Self {
            bitcoin_rpc_url,
            bitcoin_rpc_user,
            bitcoin_rpc_password,
            server_host,
            server_port,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();
    
    log::info!("Starting Esplora Mock Server...");
    
    // Load configuration
    let config = Config::from_env()
        .context("Failed to load configuration")?;
    
    log::info!("Bitcoin RPC URL: {}", config.bitcoin_rpc_url);
    log::info!("Server will listen on {}:{}", config.server_host, config.server_port);
    
    // Create Bitcoin RPC client
    let rpc_client = Arc::new(
        BitcoinRpcClient::new(
            config.bitcoin_rpc_url,
            config.bitcoin_rpc_user,
            config.bitcoin_rpc_password,
        )
        .context("Failed to create Bitcoin RPC client")?
    );
    
    // Run server
    run_server(rpc_client, config.server_host, config.server_port)
        .await
        .context("Server error")?;
    
    Ok(())
}

