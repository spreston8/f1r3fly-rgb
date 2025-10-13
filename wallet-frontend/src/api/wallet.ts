import { apiClient } from './client';
import type {
  WalletInfo,
  WalletMetadata,
  AddressInfo,
  NextAddressInfo,
  BalanceInfo,
  SyncResult,
  CreateWalletRequest,
  ImportWalletRequest,
  CreateUtxoRequest,
  CreateUtxoResponse,
  UnlockUtxoRequest,
  UnlockUtxoResponse,
  SendBitcoinRequest,
  SendBitcoinResponse,
  IssueAssetRequest,
  IssueAssetResponse,
  GenerateInvoiceRequest,
  GenerateInvoiceResponse,
  SendTransferRequest,
  SendTransferResponse,
  AcceptConsignmentResponse,
  ExportGenesisResponse,
} from './types';

export const walletApi = {
  /**
   * Create a new wallet with generated mnemonic
   */
  createWallet: async (name: string): Promise<WalletInfo> => {
    const request: CreateWalletRequest = { name };
    const response = await apiClient.post<WalletInfo>('/wallet/create', request);
    return response.data;
  },

  /**
   * Import an existing wallet from mnemonic
   */
  importWallet: async (name: string, mnemonic: string): Promise<WalletInfo> => {
    const request: ImportWalletRequest = { name, mnemonic };
    const response = await apiClient.post<WalletInfo>('/wallet/import', request);
    return response.data;
  },

  /**
   * List all wallets
   */
  listWallets: async (): Promise<WalletMetadata[]> => {
    const response = await apiClient.get<WalletMetadata[]>('/wallet/list');
    return response.data;
  },

  /**
   * Get addresses for a wallet
   */
  getAddresses: async (name: string, count: number = 10): Promise<AddressInfo[]> => {
    const response = await apiClient.get<AddressInfo[]>(`/wallet/${name}/addresses`, {
      params: { count },
    });
    return response.data;
  },

  /**
   * Get primary receive address for a wallet (always Address #0)
   */
  getPrimaryAddress: async (name: string): Promise<NextAddressInfo> => {
    const response = await apiClient.get<NextAddressInfo>(`/wallet/${name}/primary-address`);
    return response.data;
  },

  /**
   * Get balance for a wallet
   */
  getBalance: async (name: string): Promise<BalanceInfo> => {
    const response = await apiClient.get<BalanceInfo>(`/wallet/${name}/balance`);
    return response.data;
  },

  /**
   * Sync wallet with blockchain
   */
  syncWallet: async (name: string): Promise<SyncResult> => {
    const response = await apiClient.post<SyncResult>(`/wallet/${name}/sync`);
    return response.data;
  },

  /**
   * Sync RGB runtime with blockchain (updates RGB contract states)
   */
  syncRgb: async (name: string): Promise<void> => {
    await apiClient.post(`/wallet/${name}/sync-rgb`);
  },

  /**
   * Create a new RGB-compatible UTXO
   */
  createUtxo: async (
    name: string,
    request: CreateUtxoRequest
  ): Promise<CreateUtxoResponse> => {
    const response = await apiClient.post<CreateUtxoResponse>(
      `/wallet/${name}/create-utxo`,
      request
    );
    return response.data;
  },

  /**
   * Unlock a UTXO (send entire amount minus fee to a new address)
   */
  unlockUtxo: async (
    name: string,
    request: UnlockUtxoRequest
  ): Promise<UnlockUtxoResponse> => {
    const response = await apiClient.post<UnlockUtxoResponse>(
      `/wallet/${name}/unlock-utxo`,
      request
    );
    return response.data;
  },

  /**
   * Send Bitcoin to an address
   */
  sendBitcoin: async (
    name: string,
    request: SendBitcoinRequest
  ): Promise<SendBitcoinResponse> => {
    const response = await apiClient.post<SendBitcoinResponse>(
      `/wallet/${name}/send-bitcoin`,
      request
    );
    return response.data;
  },

  /**
   * Issue a new RGB20 asset
   */
  issueAsset: async (
    name: string,
    request: IssueAssetRequest
  ): Promise<IssueAssetResponse> => {
    const response = await apiClient.post<IssueAssetResponse>(
      `/wallet/${name}/issue-asset`,
      request
    );
    return response.data;
  },

  /**
   * Generate RGB invoice for receiving assets
   */
  generateInvoice: async (
    name: string,
    request: GenerateInvoiceRequest
  ): Promise<GenerateInvoiceResponse> => {
    const response = await apiClient.post<GenerateInvoiceResponse>(
      `/wallet/${name}/generate-invoice`,
      request
    );
    return response.data;
  },

  /**
   * Send RGB transfer using an invoice
   */
  sendTransfer: async (
    name: string,
    request: SendTransferRequest
  ): Promise<SendTransferResponse> => {
    const response = await apiClient.post<SendTransferResponse>(
      `/wallet/${name}/send-transfer`,
      request
    );
    return response.data;
  },

  /**
   * Accept RGB consignment (genesis or transfer)
   */
  acceptConsignment: async (
    name: string,
    consignmentBytes: Uint8Array
  ): Promise<AcceptConsignmentResponse> => {
    const response = await apiClient.post<AcceptConsignmentResponse>(
      `/wallet/${name}/accept-consignment`,
      consignmentBytes,
      {
        headers: {
          'Content-Type': 'application/octet-stream',
        },
      }
    );
    return response.data;
  },

  /**
   * Export genesis consignment for same-wallet sync
   */
  exportGenesis: async (
    name: string,
    contractId: string
  ): Promise<ExportGenesisResponse> => {
    const response = await apiClient.get<ExportGenesisResponse>(
      `/wallet/${name}/export-genesis/${contractId}`
    );
    return response.data;
  },
};

