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
  public_address: string;
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

export interface KnownContract {
  contract_id: string;
  ticker: string;
  name: string;
  balance: number;
}

export interface BalanceInfo {
  confirmed_sats: number;
  unconfirmed_sats: number;
  utxo_count: number;
  utxos: UTXO[];
  known_contracts: KnownContract[];
  display_address: string;
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

export interface SendBitcoinRequest {
  to_address: string;
  amount_sats: number;
  fee_rate_sat_vb?: number;
}

export interface SendBitcoinResponse {
  txid: string;
  amount_sats: number;
  fee_sats: number;
  to_address: string;
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

export interface IssueAssetResponseWithFirefly {
  contract_id: string;
  genesis_seal: string;
  firefly_deploy_id: string;
  firefly_block_hash: string;
  firefly_contract_data: {
    status: string;
    message: string;
    deploy_id: string;
    block_hash: string;
    asset_name: string;
    ticker: string;
    supply: number;
    precision: number;
    genesis_utxo: string;
    timestamp: number;
    contract_type: string;
  };
}

export interface GenerateInvoiceRequest {
  contract_id: string;
  amount: number;  // Required (Backend is optional)
  utxo_selection?: UtxoSelection;
  nonce?: number;
}

export type UtxoSelection =
  | { type: 'auto' }
  | { type: 'specific'; txid: string; vout: number };

export interface UtxoInfo {
  txid: string;
  vout: number;
  amount_sats: number;
  address: string;
  confirmations: number;
}

export interface GenerateInvoiceResponse {
  invoice: string;
  contract_id: string;
  amount?: number;
  seal_utxo: string;
  selected_utxo?: UtxoInfo;
}

export interface SendTransferRequest {
  invoice: string;
  fee_rate_sat_vb?: number;
}

export interface SendTransferResponse {
  bitcoin_txid: string;
  consignment_download_url: string;
  consignment_filename: string;
  status: string;
}

export interface AcceptConsignmentResponse {
  contract_id: string;
  status: string;  // "imported", "pending", or "confirmed"
  import_type: string;  // "genesis", "transfer", or "unknown"
  bitcoin_txid: string | null;
}

export interface ExportGenesisResponse {
  contract_id: string;
  consignment_filename: string;
  file_size_bytes: number;
  download_url: string;
}

export interface DeleteWalletResponse {
  wallet_name: string;
  status: string;
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

// Firefly integration types
export interface FireflyNodeStatus {
  node_connected: boolean;
  node_url: string;
  peers: number | null;
  version: string | null;
  message: string;
}

