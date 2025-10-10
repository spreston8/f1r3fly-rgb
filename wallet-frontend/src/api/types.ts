// Request types
export interface CreateWalletRequest {
  name: string;
}

export interface ImportWalletRequest {
  name: string;
  mnemonic: string;
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

export interface UTXO {
  txid: string;
  vout: number;
  amount_sats: number;
  address: string;
  confirmations: number;
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

// Error response type
export interface ApiError {
  error: string;
}

