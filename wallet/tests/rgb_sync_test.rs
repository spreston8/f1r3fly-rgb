use bpstd::Network;
use std::path::PathBuf;
use std::time::Instant;
use wallet::storage::KeyManager;
use wallet::rgb::runtime::RgbRuntimeManager;

/// Test helper that ensures cleanup of test directories
/// Uses RAII pattern - cleanup happens when this struct is dropped
struct TestWalletDir {
    path: PathBuf,
}

impl TestWalletDir {
    fn new(name: &str) -> Self {
        let path = PathBuf::from(format!("./test_wallets_{}", name));

        // Remove if already exists (from previous failed test)
        if path.exists() {
            log::warn!("Cleaning up existing test directory: {:?}", path);
            std::fs::remove_dir_all(&path).ok();
        }

        // Create fresh directory
        std::fs::create_dir_all(&path).expect("Failed to create test directory");

        log::debug!("Created test directory: {:?}", path);

        Self { path }
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Create a minimal wallet structure for RGB runtime testing
    fn create_test_wallet(&self, wallet_name: &str) {
        let wallet_path = self.path.join(wallet_name);
        std::fs::create_dir_all(&wallet_path).expect("Failed to create wallet directory");

        // Generate wallet keys
        let keys = KeyManager::generate().expect("Failed to generate wallet keys");

        // Write descriptor file (required by RGB runtime)
        let descriptor_path = wallet_path.join("descriptor.txt");
        std::fs::write(&descriptor_path, &keys.descriptor).expect("Failed to write descriptor");

        // Create RGB data directory (stockpile directory)
        let rgb_data_path = wallet_path.join("rgb_data");
        std::fs::create_dir_all(&rgb_data_path).expect("Failed to create RGB data directory");

        log::debug!("Created test wallet structure at: {:?}", wallet_path);
        log::debug!("Descriptor: {}", keys.descriptor);
    }
}

impl Drop for TestWalletDir {
    fn drop(&mut self) {
        log::debug!("Cleaning up test directory: {:?}", self.path);
        if let Err(e) = std::fs::remove_dir_all(&self.path) {
            log::warn!("Failed to cleanup test directory {:?}: {}", self.path, e);
        } else {
            log::debug!("Successfully cleaned up test directory");
        }
    }
}

#[test]
fn test_rgb_sync_duration() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .filter_module("wallet", log::LevelFilter::Debug)
        .is_test(true)
        .try_init()
        .ok();

    log::info!("=== Starting RGB Sync Duration Test ===");

    // TestWalletDir will automatically cleanup on drop (even if test fails)
    let test_dir = TestWalletDir::new("rgb_sync");
    let wallet_name = "sync-test-wallet";

    // Create minimal wallet structure
    test_dir.create_test_wallet(wallet_name);

    let rgb_runtime_manager = RgbRuntimeManager::new(
        test_dir.path().clone(),
        Network::Signet,
        "https://mempool.space/signet/api".to_string(),
    );

    log::info!("Initializing RGB runtime for wallet: {}", wallet_name);
    let mut runtime = rgb_runtime_manager
        .init_runtime_no_sync(wallet_name)
        .expect("Failed to initialize RGB runtime");

    log::info!("Starting sync measurement...");
    log::info!("This will scan wallet addresses and query Esplora API...");
    let start = Instant::now();

    // Sync with 1 confirmation (same as transfer sync)
    let result = runtime.update(1);

    let duration = start.elapsed();

    log::info!("=== Sync completed in {:?} ===", duration);

    assert!(result.is_ok(), "Sync should not error: {:?}", result.err());

    let duration_secs = duration.as_secs();
    log::info!("Sync duration: {} seconds", duration_secs);

    assert!(
        duration_secs < 30,
        "Sync should complete within 30 seconds, but took {} seconds",
        duration_secs
    );

    log::info!("✓ Test passed - sync completed in acceptable time");

    // test_dir will be automatically cleaned up here when it goes out of scope
}
