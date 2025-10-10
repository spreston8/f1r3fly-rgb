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
};

