use wallet::api::server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "127.0.0.1:3000";
    println!("Starting RGB-compatible Bitcoin wallet server on {}", addr);
    server::start_server(addr).await?;
    Ok(())
}

