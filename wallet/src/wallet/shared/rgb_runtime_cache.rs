/// RGB Runtime Cache - Long-Lived Runtime Management
/// 
/// Maintains RGB runtime instances across API requests to preserve internal state
/// (especially seals in the pile) that would otherwise be lost with ephemeral runtimes.

use bpstd::seals::TxoSeal;
use bpstd::{Network, Wpkh, XpubDerivable};
use hypersonic::Consensus;
use rgb::popls::bp::RgbWallet;
use rgb::Contracts;
use rgb_descriptors::RgbDescr;
use rgb_persist_fs::StockpileDir;
use rgbp::resolvers::MultiResolver;
use rgbp::{FileHolder, Owner, RgbpRuntimeDir};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use crate::error::WalletError;

/// RGB Runtime Cache - manages long-lived runtime instances
pub struct RgbRuntimeCache {
    /// Cached runtimes per wallet
    runtimes: Arc<RwLock<HashMap<String, Arc<Mutex<CachedRuntime>>>>>,
    /// Base path for wallet data
    base_path: PathBuf,
    /// Bitcoin network
    network: Network,
    /// Esplora API URL
    esplora_url: String,
}

/// Cached runtime with metadata
struct CachedRuntime {
    runtime: RgbpRuntimeDir<MultiResolver>,
    last_access: Instant,
    last_save: Instant,
    dirty: bool,
}

impl RgbRuntimeCache {
    pub fn new(base_path: PathBuf, network: Network, esplora_url: String) -> Self {
        log::info!("Initializing RGB runtime cache");
        Self {
            runtimes: Arc::new(RwLock::new(HashMap::new())),
            base_path,
            network,
            esplora_url,
        }
    }

    /// Get or create a runtime for a wallet
    pub fn get_or_create(&self, wallet_name: &str) -> Result<RuntimeGuard, WalletError> {
        // Fast path: check if runtime exists (read lock)
        {
            let cache = self.runtimes.read().unwrap();
            if let Some(runtime) = cache.get(wallet_name) {
                let mut rt = runtime.lock().unwrap();
                rt.last_access = Instant::now();
                log::debug!("RGB runtime cache hit for wallet: {}", wallet_name);
                return Ok(RuntimeGuard {
                    runtime: runtime.clone(),
                    wallet_name: wallet_name.to_string(),
                });
            }
        }

        // Slow path: create runtime (write lock)
        let mut cache = self.runtimes.write().unwrap();

        // Double-check pattern (another thread might have created it)
        if let Some(runtime) = cache.get(wallet_name) {
            log::debug!("RGB runtime cache hit after write lock for wallet: {}", wallet_name);
            return Ok(RuntimeGuard {
                runtime: runtime.clone(),
                wallet_name: wallet_name.to_string(),
            });
        }

        // Create new runtime
        log::debug!("RGB runtime cache miss - creating new runtime for wallet: {}", wallet_name);
        let runtime = self.create_runtime(wallet_name)?;
        let cached = Arc::new(Mutex::new(CachedRuntime {
            runtime,
            last_access: Instant::now(),
            last_save: Instant::now(),
            dirty: false,
        }));

        cache.insert(wallet_name.to_string(), cached.clone());
        log::info!("Created and cached RGB runtime for wallet: {}", wallet_name);

        Ok(RuntimeGuard {
            runtime: cached,
            wallet_name: wallet_name.to_string(),
        })
    }

    /// Create a new RGB runtime (extracted from RgbRuntimeManager)
    fn create_runtime(
        &self,
        wallet_name: &str,
    ) -> Result<RgbpRuntimeDir<MultiResolver>, WalletError> {
        // 1. Create resolver
        let resolver = self.create_resolver()?;

        // 2. Ensure FileHolder exists (create if needed)
        let rgb_wallet_path = self.base_path.join(wallet_name).join("rgb_wallet");

        let hodler = if rgb_wallet_path.exists() {
            FileHolder::load(rgb_wallet_path)
                .map_err(|e| WalletError::Rgb(e.to_string()))?
        } else {
            self.create_file_holder(wallet_name)?
        };

        // 3. Create Owner
        let owner = Owner::with_components(self.network, hodler, resolver);

        // 4. Load Contracts (per-wallet RGB data)
        let contracts = self.load_contracts(wallet_name)?;

        // 5. Create RgbWallet
        let rgb_wallet = RgbWallet::with_components(owner, contracts);

        // 6. Wrap in RgbRuntime and return
        let runtime = RgbpRuntimeDir::from(rgb_wallet);

        Ok(runtime)
    }

    fn create_resolver(&self) -> Result<MultiResolver, WalletError> {
        MultiResolver::new_esplora(&self.esplora_url)
            .map_err(|e| WalletError::Network(e.to_string()))
    }

    fn create_file_holder(&self, wallet_name: &str) -> Result<FileHolder, WalletError> {
        // Load our descriptor
        let descriptor_path = self.base_path.join(wallet_name).join("descriptor.txt");
        let descriptor_str = std::fs::read_to_string(&descriptor_path)
            .map_err(|e| WalletError::Rgb(format!("Failed to load descriptor: {}", e)))?;

        // Convert to RgbDescr
        let rgb_descr = self.descriptor_to_rgb(&descriptor_str)?;

        // Create FileHolder directory
        let rgb_wallet_path = self.base_path.join(wallet_name).join("rgb_wallet");

        FileHolder::create(rgb_wallet_path, rgb_descr)
            .map_err(|e| WalletError::Rgb(e.to_string()))
    }

    fn descriptor_to_rgb(&self, descriptor: &str) -> Result<RgbDescr, WalletError> {
        let xpub = XpubDerivable::from_str(descriptor)
            .map_err(|e| WalletError::InvalidDescriptor(e.to_string()))?;

        let noise = xpub.xpub().chain_code().to_byte_array();

        Ok(RgbDescr::new_unfunded(Wpkh::from(xpub), noise))
    }

    fn load_contracts(
        &self,
        wallet_name: &str,
    ) -> Result<Contracts<StockpileDir<TxoSeal>>, WalletError> {
        let rgb_data_dir = self.base_path.join(wallet_name).join("rgb_data");

        // Create rgb_data directory if it doesn't exist (for wallets without RGB assets yet)
        if !rgb_data_dir.exists() {
            log::debug!("Creating RGB data directory for wallet: {}", wallet_name);
            std::fs::create_dir_all(&rgb_data_dir).map_err(|e| {
                WalletError::Rgb(format!("Failed to create RGB data directory: {:?}", e))
            })?;
            log::debug!("RGB data directory created successfully");
        }

        let stockpile = StockpileDir::load(rgb_data_dir, Consensus::Bitcoin, true)
            .map_err(|e| WalletError::Rgb(format!("Failed to load stockpile: {:?}", e)))?;

        Ok(Contracts::load(stockpile))
    }

    /// Evict a runtime from cache (saves before removing)
    pub fn evict(&self, wallet_name: &str) -> Result<(), WalletError> {
        let mut cache = self.runtimes.write().unwrap();
        if let Some(runtime_arc) = cache.remove(wallet_name) {
            // Runtime will auto-save descriptor on drop
            let rt = runtime_arc.lock().unwrap();
            log::info!("Evicted RGB runtime for wallet: {} (last accessed: {:?} ago)", 
                wallet_name, rt.last_access.elapsed());
            drop(rt); // Explicit drop for clarity
        }
        Ok(())
    }

    /// Save all dirty runtimes
    pub fn save_all(&self) -> Result<(), WalletError> {
        let cache = self.runtimes.read().unwrap();
        let mut saved_count = 0;
        let mut error_count = 0;

        for (name, runtime_arc) in cache.iter() {
            let mut rt = runtime_arc.lock().unwrap();
            if rt.dirty {
                // Call update(1) to persist state
                match rt.runtime.update(1) {
                    Ok(_) => {
                        rt.last_save = Instant::now();
                        rt.dirty = false;
                        saved_count += 1;
                        log::debug!("Saved RGB runtime for wallet: {}", name);
                    }
                    Err(e) => {
                        error_count += 1;
                        log::error!("Failed to save RGB runtime for wallet {}: {:?}", name, e);
                    }
                }
            }
        }

        if saved_count > 0 {
            log::info!("Auto-saved {} RGB runtime(s)", saved_count);
        }
        if error_count > 0 {
            log::error!("Failed to save {} RGB runtime(s)", error_count);
        }

        Ok(())
    }

    /// Get current cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.runtimes.read().unwrap();
        let mut dirty_count = 0;
        let mut oldest_access = None;

        for runtime_arc in cache.values() {
            let rt = runtime_arc.lock().unwrap();
            if rt.dirty {
                dirty_count += 1;
            }
            if oldest_access.is_none() || rt.last_access < oldest_access.unwrap() {
                oldest_access = Some(rt.last_access);
            }
        }

        CacheStats {
            total_cached: cache.len(),
            dirty_count,
            oldest_access,
        }
    }

    /// Clean up idle runtimes
    /// 
    /// Evicts runtimes that haven't been accessed recently, respecting max cache size.
    /// Returns the number of runtimes evicted.
    pub fn cleanup_idle(
        &self,
        idle_timeout: std::time::Duration,
        max_cached: usize,
    ) -> Result<usize, WalletError> {
        let now = Instant::now();
        let mut to_evict = Vec::new();

        // Phase 1: Identify idle runtimes
        {
            let cache = self.runtimes.read().unwrap();
            
            for (name, runtime_arc) in cache.iter() {
                let rt = runtime_arc.lock().unwrap();
                let age = now.duration_since(rt.last_access);
                
                if age > idle_timeout {
                    to_evict.push((name.clone(), age));
                    log::debug!(
                        "Runtime {} is idle (last accessed {:?} ago)",
                        name,
                        age
                    );
                }
            }

            // Phase 2: LRU eviction if over max_cached
            if cache.len() > max_cached {
                let mut entries: Vec<(String, Instant)> = cache
                    .iter()
                    .map(|(name, runtime_arc)| {
                        let rt = runtime_arc.lock().unwrap();
                        (name.clone(), rt.last_access)
                    })
                    .collect();

                // Sort by last_access (oldest first)
                entries.sort_by_key(|(_, last_access)| *last_access);

                // Mark oldest entries for eviction
                let to_remove = cache.len() - max_cached;
                for (name, last_access) in entries.iter().take(to_remove) {
                    let age = now.duration_since(*last_access);
                    if !to_evict.iter().any(|(n, _)| n == name) {
                        to_evict.push((name.clone(), age));
                        log::debug!(
                            "Runtime {} marked for LRU eviction (last accessed {:?} ago)",
                            name,
                            age
                        );
                    }
                }
            }
        }

        // Phase 3: Evict identified runtimes
        let evicted_count = to_evict.len();
        for (name, age) in to_evict {
            self.evict(&name)?;
            log::info!(
                "Evicted idle runtime: {} (idle for {:?})",
                name,
                age
            );
        }

        Ok(evicted_count)
    }
}

/// RAII guard for runtime access
pub struct RuntimeGuard {
    runtime: Arc<Mutex<CachedRuntime>>,
    wallet_name: String,
}

impl RuntimeGuard {
    /// Execute an operation on the cached runtime
    pub fn execute<F, R>(&self, f: F) -> Result<R, WalletError>
    where
        F: FnOnce(&mut RgbpRuntimeDir<MultiResolver>) -> Result<R, WalletError>,
    {
        let mut rt = self.runtime.lock().unwrap();
        let result = f(&mut rt.runtime)?;
        rt.dirty = true;
        rt.last_access = Instant::now();
        Ok(result)
    }

    /// Get the wallet name this guard is for
    pub fn wallet_name(&self) -> &str {
        &self.wallet_name
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_cached: usize,
    pub dirty_count: usize,
    pub oldest_access: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = RgbRuntimeCache::new(
            temp_dir.path().to_path_buf(),
            Network::Signet,
            "https://mempool.space/signet/api".to_string(),
        );
        
        let stats = cache.stats();
        assert_eq!(stats.total_cached, 0);
        assert_eq!(stats.dirty_count, 0);
    }

    #[test]
    fn test_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cache = RgbRuntimeCache::new(
            temp_dir.path().to_path_buf(),
            Network::Signet,
            "https://mempool.space/signet/api".to_string(),
        );
        
        let stats = cache.stats();
        assert_eq!(stats.total_cached, 0);
    }
}

