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

export interface IssueAssetRequest {
  name: string;           // 2-12 chars
  ticker: string;         // 2-8 chars
  precision: number;      // 0-10
  supply: number;         // Total supply
  genesis_utxo: string;   // "txid:vout"
}

export interface IssueAssetResponse {
  contract_id: string;
  genesis_seal: string;
}

// Precision options for RGB20 assets
export const PRECISION_OPTIONS = [
  { value: 0, label: 'Indivisible (0 decimals)', example: '1' },
  { value: 2, label: 'Centi (2 decimals)', example: '0.01' },
  { value: 8, label: 'CentiMicro (8 decimals - Like BTC)', example: '0.00000001' },
  { value: 1, label: 'Deci (1 decimal)', example: '0.1' },
  { value: 3, label: 'Milli (3 decimals)', example: '0.001' },
  { value: 4, label: 'DeciMilli (4 decimals)', example: '0.0001' },
  { value: 5, label: 'CentiMilli (5 decimals)', example: '0.00001' },
  { value: 6, label: 'Micro (6 decimals)', example: '0.000001' },
  { value: 7, label: 'DeciMicro (7 decimals)', example: '0.0000001' },
  { value: 9, label: 'Nano (9 decimals)', example: '0.000000001' },
  { value: 10, label: 'DeciNano (10 decimals)', example: '0.0000000001' },
];

// Error response type
export interface ApiError {
  error: string;
}

