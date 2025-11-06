/// RGB Runtime Lifecycle Manager
/// 
/// Manages background tasks for RGB runtime maintenance:
/// - Auto-save dirty runtimes periodically
/// - Cleanup idle runtimes to prevent memory leaks
/// - Graceful shutdown with state preservation

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

use super::cache::RgbRuntimeCache;
use crate::error::WalletError;

/// Manages RGB runtime lifecycle (auto-save, idle cleanup, shutdown)
pub struct RuntimeLifecycleManager {
    cache: Arc<RgbRuntimeCache>,
    config: LifecycleConfig,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

/// Configuration for runtime lifecycle management
#[derive(Clone, Debug)]
pub struct LifecycleConfig {
    /// Drop runtime after this much inactivity
    pub idle_timeout: Duration,
    /// Periodic save interval for dirty runtimes
    pub auto_save_interval: Duration,
    /// Maximum number of cached runtimes (LRU eviction)
    pub max_cached_runtimes: usize,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            idle_timeout: Duration::from_secs(30 * 60), // 30 minutes
            auto_save_interval: Duration::from_secs(5 * 60), // 5 minutes
            max_cached_runtimes: 100,
        }
    }
}

impl RuntimeLifecycleManager {
    /// Create a new runtime lifecycle manager with the specified cache and configuration
    pub fn new(cache: Arc<RgbRuntimeCache>, config: LifecycleConfig) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        log::info!(
            "RGB runtime lifecycle manager initialized (idle_timeout={}s, auto_save={}s, max_cached={})",
            config.idle_timeout.as_secs(),
            config.auto_save_interval.as_secs(),
            config.max_cached_runtimes
        );
        Self {
            cache,
            config,
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Start background tasks
    /// 
    /// This spawns two background tasks:
    /// 1. Auto-save loop: Periodically saves dirty runtimes
    /// 2. Idle cleanup loop: Evicts idle runtimes to prevent memory leaks
    pub async fn start(self: Arc<Self>) {
        log::info!("Starting RGB runtime lifecycle manager background tasks");

        let auto_save_handle = {
            let manager = self.clone();
            tokio::spawn(async move {
                manager.auto_save_loop().await;
            })
        };

        let idle_cleanup_handle = {
            let manager = self.clone();
            tokio::spawn(async move {
                manager.idle_cleanup_loop().await;
            })
        };

        // Wait for shutdown signal
        let mut rx = self.shutdown_rx.clone();
        let _ = rx.changed().await;

        log::info!("Shutdown signal received, stopping background tasks...");

        // Cancel background tasks
        auto_save_handle.abort();
        idle_cleanup_handle.abort();

        log::info!("RGB runtime lifecycle manager background tasks stopped");
    }

    /// Periodically save dirty runtimes at the configured auto-save interval
    async fn auto_save_loop(&self) {
        let mut interval = tokio::time::interval(self.config.auto_save_interval);
        let mut shutdown_rx = self.shutdown_rx.clone();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.cache.save_all() {
                        log::error!("Auto-save failed: {}", e);
                    }
                }
                _ = shutdown_rx.changed() => {
                    log::info!("Auto-save loop shutting down");
                    break;
                }
            }
        }
    }

    /// Periodically evict runtimes that haven't been used recently to prevent memory leaks
    async fn idle_cleanup_loop(&self) {
        // Check every 1/4 of the idle timeout
        let check_interval = self.config.idle_timeout / 4;
        let mut interval = tokio::time::interval(check_interval);
        let mut shutdown_rx = self.shutdown_rx.clone();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.cleanup_idle_runtimes();
                }
                _ = shutdown_rx.changed() => {
                    log::info!("Idle cleanup loop shutting down");
                    break;
                }
            }
        }
    }

    /// Clean up idle runtimes
    /// 
    /// Evicts runtimes that haven't been accessed recently or when cache exceeds max size.
    /// This runs periodically as part of the idle cleanup background task.
    fn cleanup_idle_runtimes(&self) {
        let stats = self.cache.stats();

        // Perform idle cleanup and LRU eviction
        match self.cache.cleanup_idle(
            self.config.idle_timeout,
            self.config.max_cached_runtimes,
        ) {
            Ok(evicted_count) => {
                if evicted_count > 0 {
                    log::info!(
                        "Idle cleanup evicted {} runtime(s), {} remaining",
                        evicted_count,
                        stats.total_cached - evicted_count
                    );
                }
            }
            Err(e) => {
                log::error!("Idle cleanup failed: {}", e);
            }
        }
    }

    /// Gracefully shutdown the lifecycle manager, stopping background tasks and saving all runtime state
    pub async fn shutdown(&self) -> Result<(), WalletError> {
        log::info!("Shutting down RGB runtime lifecycle manager...");

        // Signal background tasks to stop
        let _ = self.shutdown_tx.send(true);

        // Give tasks a moment to clean up
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Save all dirty runtimes
        log::info!("Saving all RGB runtimes...");
        self.cache.save_all()?;

        log::info!("RGB runtime lifecycle manager shutdown complete");
        Ok(())
    }

    /// Get the current lifecycle configuration
    pub fn config(&self) -> &LifecycleConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = LifecycleConfig::default();
        assert_eq!(config.idle_timeout.as_secs(), 30 * 60);
        assert_eq!(config.auto_save_interval.as_secs(), 5 * 60);
        assert_eq!(config.max_cached_runtimes, 100);
    }

    #[tokio::test]
    async fn test_lifecycle_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Arc::new(RgbRuntimeCache::new(
            temp_dir.path().to_path_buf(),
            bpstd::Network::Signet,
            "https://mempool.space/signet/api".to_string(),
        ));
        let config = LifecycleConfig::default();
        let manager = RuntimeLifecycleManager::new(cache, config);
        
        assert_eq!(manager.config().max_cached_runtimes, 100);
    }
}

