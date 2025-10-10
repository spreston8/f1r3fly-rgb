// Request types
export interface CreateWalletRequest {
  name: string;
}

export interface ImportWalletRequest {
  name: string;
  mnemonic: string;
}

export interface CreateUtxoRequest {
  amount_btc?: number;
  fee_rate_sat_vb?: number;
}

// Response types
export interface WalletInfo {
  name: string;
  mnemonic: string;
  first_address: string;
  descriptor: string;
}

export interface WalletMetadata {
  name: string;
  created_at: string;
  last_synced?: string;
}

export interface AddressInfo {
  index: number;
  address: string;
  used: boolean;
}

export interface NextAddressInfo {
  address: string;
  index: number;
  total_used: number;
  descriptor: string;
}

export interface BoundAsset {
  asset_id: string;
  asset_name: string;
  ticker: string;
  amount: string;
}

export interface UTXO {
  txid: string;
  vout: number;
  amount_sats: number;
  address: string;
  confirmations: number;
  is_occupied: boolean;
  bound_assets: BoundAsset[];
}

export interface BalanceInfo {
  confirmed_sats: number;
  unconfirmed_sats: number;
  utxo_count: number;
  utxos: UTXO[];
}

export interface SyncResult {
  synced_height: number;
  addresses_checked: number;
  new_transactions: number;
}

export interface CreateUtxoResponse {
  txid: string;
  amount_sats: number;
  fee_sats: number;
  target_address: string;
}

export interface UnlockUtxoRequest {
  txid: string;
  vout: number;
  fee_rate_sat_vb?: number;
}

export interface UnlockUtxoResponse {
  txid: string;
  recovered_sats: number;
  fee_sats: number;
}

// Error response type
export interface ApiError {
  error: string;
}

