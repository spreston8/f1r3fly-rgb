use std::path::PathBuf;
use std::str::FromStr;
use bpstd::{Network, XpubDerivable, Wpkh};
use rgbp::{Owner, FileHolder, RgbpRuntimeDir};
use rgbp::resolvers::MultiResolver;
use rgb_descriptors::RgbDescr;
use rgb::{Contracts, Consensus};
use rgb_persist_fs::StockpileDir;
use bpstd::seals::TxoSeal;
use rgb::popls::bp::RgbWallet;

pub struct RgbRuntimeManager {
    base_path: PathBuf,
    network: Network,
}

impl RgbRuntimeManager {
    pub fn new(base_path: PathBuf, network: Network) -> Self {
        Self { base_path, network }
    }
    
    /// Initialize RGB Runtime for a specific wallet (with blockchain sync)
    pub fn init_runtime(
        &self,
        wallet_name: &str,
    ) -> Result<RgbpRuntimeDir<MultiResolver>, crate::error::WalletError> {
        self.init_runtime_internal(wallet_name, true)
    }
    
    /// Initialize RGB Runtime without blockchain sync (fast, for invoice generation)
    pub fn init_runtime_no_sync(
        &self,
        wallet_name: &str,
    ) -> Result<RgbpRuntimeDir<MultiResolver>, crate::error::WalletError> {
        self.init_runtime_internal(wallet_name, false)
    }
    
    /// Internal runtime initialization with optional sync
    fn init_runtime_internal(
        &self,
        wallet_name: &str,
        sync: bool,
    ) -> Result<RgbpRuntimeDir<MultiResolver>, crate::error::WalletError> {
        // 1. Create resolver
        let resolver = self.create_resolver()?;
        
        // 2. Ensure FileHolder exists (create if needed)
        let rgb_wallet_path = self.base_path
            .join(wallet_name)
            .join("rgb_wallet");
        
        let hodler = if rgb_wallet_path.exists() {
            FileHolder::load(rgb_wallet_path)
                .map_err(|e| crate::error::WalletError::Rgb(e.to_string()))?
        } else {
            self.create_file_holder(wallet_name)?
        };
        
        // 3. Create Owner
        let owner = Owner::with_components(self.network, hodler, resolver);
        
        // 4. Load Contracts (shared RGB data)
        let contracts = self.load_contracts()?;
        
        // 5. Create RgbWallet
        let rgb_wallet = RgbWallet::with_components(owner, contracts);
        
        // 6. Wrap in RgbRuntime
        let mut runtime = RgbpRuntimeDir::from(rgb_wallet);
        
        // 7. Optionally sync wallet with blockchain (slow, skip for invoice generation)
        if sync {
            runtime.update(32)
                .map_err(|e| crate::error::WalletError::Rgb(format!("Sync failed: {:?}", e)))?;
        }
        
        Ok(runtime)
    }
    
    fn create_resolver(&self) -> Result<MultiResolver, crate::error::WalletError> {
        MultiResolver::new_esplora("https://mempool.space/signet/api")
            .map_err(|e| crate::error::WalletError::Network(e.to_string()))
    }
    
    fn create_file_holder(
        &self,
        wallet_name: &str,
    ) -> Result<FileHolder, crate::error::WalletError> {
        // Load our descriptor
        let descriptor_path = self.base_path
            .join(wallet_name)
            .join("descriptor.txt");
        let descriptor_str = std::fs::read_to_string(&descriptor_path)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to load descriptor: {}", e)))?;
        
        // Convert to RgbDescr
        let rgb_descr = self.descriptor_to_rgb(&descriptor_str)?;
        
        // Create FileHolder directory
        let rgb_wallet_path = self.base_path
            .join(wallet_name)
            .join("rgb_wallet");
        
        FileHolder::create(rgb_wallet_path, rgb_descr)
            .map_err(|e| crate::error::WalletError::Rgb(e.to_string()))
    }
    
    fn descriptor_to_rgb(
        &self,
        descriptor: &str,
    ) -> Result<RgbDescr, crate::error::WalletError> {
        let xpub = XpubDerivable::from_str(descriptor)
            .map_err(|e| crate::error::WalletError::InvalidDescriptor(e.to_string()))?;
        
        let noise = xpub.xpub().chain_code().to_byte_array();
        
        Ok(RgbDescr::new_unfunded(Wpkh::from(xpub), noise))
    }
    
    fn load_contracts(&self) -> Result<Contracts<StockpileDir<TxoSeal>>, crate::error::WalletError> {
        let rgb_data_dir = self.base_path.join("rgb_data");
        let stockpile = StockpileDir::load(rgb_data_dir, Consensus::Bitcoin, true)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to load stockpile: {:?}", e)))?;
        
        Ok(Contracts::load(stockpile))
    }
}

