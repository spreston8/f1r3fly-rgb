import { useState, useEffect, useCallback } from 'react';
import { Link, useParams, useNavigate } from 'react-router-dom';
import { walletApi } from '../api';
import type { BalanceInfo, AddressInfo } from '../api/types';
import BalanceDisplay from '../components/BalanceDisplay';
import AddressList from '../components/AddressList';
import UTXOList from '../components/UTXOList';
import CreateUtxoModal from '../components/CreateUtxoModal';
import IssueAssetModal from '../components/IssueAssetModal';
import GenerateInvoiceModal from '../components/GenerateInvoiceModal';
import SendTransferModal from '../components/SendTransferModal';
import AcceptConsignmentModal from '../components/AcceptConsignmentModal';
import ExportGenesisModal from '../components/ExportGenesisModal';
import SendBitcoinModal from '../components/SendBitcoinModal';
import DeleteWalletModal from '../components/DeleteWalletModal';
import { copyToClipboard } from '../utils/format';

export default function WalletDetail() {
  const { name } = useParams<{ name: string }>();
  const navigate = useNavigate();
  const [balance, setBalance] = useState<BalanceInfo | null>(null);
  const [addresses, setAddresses] = useState<AddressInfo[]>([]);
  const [showAllAddresses, setShowAllAddresses] = useState(false);
  const [addressCount, setAddressCount] = useState(20);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [syncingRgb, setSyncingRgb] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [addressCopied, setAddressCopied] = useState(false);
  const [descriptorCopied, setDescriptorCopied] = useState(false);
  const [descriptor, setDescriptor] = useState<string>('');
  const [showCreateUtxoModal, setShowCreateUtxoModal] = useState(false);
  const [showIssueAssetModal, setShowIssueAssetModal] = useState(false);
  const [showGenerateInvoiceModal, setShowGenerateInvoiceModal] = useState(false);
  const [showSendTransferModal, setShowSendTransferModal] = useState(false);
  const [showAcceptConsignmentModal, setShowAcceptConsignmentModal] = useState(false);
  const [showExportGenesisModal, setShowExportGenesisModal] = useState(false);
  const [showSendBitcoinModal, setShowSendBitcoinModal] = useState(false);
  const [showDeleteWalletModal, setShowDeleteWalletModal] = useState(false);
  const [selectedAsset, setSelectedAsset] = useState<{ contractId: string; ticker: string } | null>(null);
  const [selectedAssetForExport, setSelectedAssetForExport] = useState<{ contractId: string; ticker: string } | null>(null);

  const loadWalletData = useCallback(async () => {
    if (!name) return;

    try {
      setLoading(true);
      setError(null);
      
      const [balanceData, nextAddressData] = await Promise.all([
        walletApi.getBalance(name),
        walletApi.getPrimaryAddress(name),
      ]);
      
      setBalance(balanceData);
      setDescriptor(nextAddressData.descriptor);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load wallet data');
    } finally {
      setLoading(false);
    }
  }, [name]);

  useEffect(() => {
    if (name) {
      loadWalletData();
    }
  }, [name, loadWalletData]);

  const loadAllAddresses = async () => {
    if (!name) return;

    try {
      const addressData = await walletApi.getAddresses(name, addressCount);
      setAddresses(addressData);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load addresses');
    }
  };

  const handleSync = async () => {
    if (!name) return;

    try {
      setSyncing(true);
      await walletApi.syncWallet(name);
      await loadWalletData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to sync wallet');
    } finally {
      setSyncing(false);
    }
  };

  const handleCopyAddress = async () => {
    if (balance?.display_address) {
      const success = await copyToClipboard(balance.display_address);
      if (success) {
        setAddressCopied(true);
        setTimeout(() => setAddressCopied(false), 2000);
      }
    }
  };

  const handleCopyDescriptor = async () => {
    if (descriptor) {
      const success = await copyToClipboard(descriptor);
      if (success) {
        setDescriptorCopied(true);
        setTimeout(() => setDescriptorCopied(false), 2000);
      }
    }
  };

  const handleToggleAllAddresses = () => {
    if (!showAllAddresses && addresses.length === 0) {
      loadAllAddresses();
    }
    setShowAllAddresses(!showAllAddresses);
  };

  if (loading) {
    return (
      <div className="space-y-6">
        <Link to="/" className="text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200">
          ← Back to Wallets
        </Link>
        <div className="bg-white dark:bg-gray-800 shadow dark:shadow-gray-900 rounded-lg p-6">
          <div className="flex justify-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
          </div>
        </div>
      </div>
    );
  }

  if (error && !balance) {
    return (
      <div className="space-y-6">
        <Link to="/" className="text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200">
          ← Back to Wallets
        </Link>
        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
          <p className="text-red-800 dark:text-red-300">Error: {error}</p>
          <button
            onClick={loadWalletData}
            className="mt-2 text-sm text-red-600 dark:text-red-400 hover:text-red-800 dark:hover:text-red-300"
          >
            Try again
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4">
        <Link to="/" className="text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200">
          ← Back to Wallets
        </Link>
      </div>

      <div className="flex items-center justify-between">
        <h2 className="text-3xl font-bold text-gray-900 dark:text-white">💰 {name}</h2>
        <div className="flex gap-2">
          <button
            onClick={() => setShowAcceptConsignmentModal(true)}
            className="px-4 py-2 bg-purple-600 hover:bg-purple-700 dark:bg-purple-500 dark:hover:bg-purple-600 text-white rounded-md transition-colors font-medium"
          >
            📥 Import Consignment
          </button>
          <button
            onClick={() => setShowSendBitcoinModal(true)}
            className="px-4 py-2 bg-orange-600 hover:bg-orange-700 dark:bg-orange-500 dark:hover:bg-orange-600 text-white rounded-md transition-colors font-medium"
          >
            💸 Send Bitcoin
          </button>
          <button
            onClick={() => setShowCreateUtxoModal(true)}
            className="px-4 py-2 bg-green-600 hover:bg-green-700 dark:bg-green-500 dark:hover:bg-green-600 text-white rounded-md transition-colors font-medium"
          >
            ➕ Create UTXO
          </button>
          <button
            onClick={() => setShowIssueAssetModal(true)}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600 text-white rounded-md transition-colors font-medium"
          >
            🪙 Issue Asset
          </button>
        </div>
      </div>

      {error && (
        <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700 rounded-lg p-4">
          <p className="text-yellow-800 dark:text-yellow-300">⚠️ {error}</p>
        </div>
      )}

      {balance && (
        <BalanceDisplay
          balance={balance}
          onSync={handleSync}
          syncing={syncing}
        />
      )}

      {/* RGB Assets Section */}
      {balance && balance.known_contracts && balance.known_contracts.length > 0 && (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
              🪙 RGB Assets
            </h3>
            {syncingRgb && (
              <div className="flex items-center gap-2 text-sm text-blue-600 dark:text-blue-400">
                <svg className="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span>Syncing RGB state...</span>
              </div>
            )}
          </div>
          <div className="space-y-3">
            {balance.known_contracts.map((contract) => (
              <div
                key={contract.contract_id}
                className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-700 rounded-lg border border-gray-200 dark:border-gray-600"
              >
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <span className="px-2 py-1 bg-orange-100 dark:bg-orange-900/30 text-orange-800 dark:text-orange-300 text-xs font-semibold rounded">
                        {contract.ticker}
                      </span>
                      <h4 className="font-medium text-gray-900 dark:text-white">
                        {contract.name}
                      </h4>
                      {contract.balance === 0 && (
                        <span className="px-2 py-1 bg-gray-200 dark:bg-gray-600 text-gray-600 dark:text-gray-300 text-xs rounded">
                          Known Contract
                        </span>
                      )}
                    </div>
                    <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                      Balance: {contract.balance.toString()}
                    </p>
                    {contract.balance === 0 && (
                      <p className="text-xs text-yellow-600 dark:text-yellow-400 mt-1">
                        ℹ️ Need Bitcoin UTXOs to receive tokens
                      </p>
                    )}
                    <p className="text-xs text-gray-500 dark:text-gray-500 mt-1 font-mono">
                      {contract.contract_id}
                    </p>
                  </div>
                <div className="flex gap-2">
                  <button
                    onClick={() => setShowSendTransferModal(true)}
                    disabled={contract.balance === 0}
                    className={`px-4 py-2 ${
                      contract.balance === 0
                        ? 'bg-gray-400 dark:bg-gray-600 cursor-not-allowed'
                        : 'bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600'
                    } text-white rounded-md transition-colors font-medium text-sm`}
                    title={contract.balance === 0 ? 'No balance to send' : 'Send tokens'}
                  >
                    📤 Send
                  </button>
                  <button
                    onClick={() => {
                      setSelectedAsset({ contractId: contract.contract_id, ticker: contract.ticker });
                      setShowGenerateInvoiceModal(true);
                    }}
                    className="px-4 py-2 bg-green-600 hover:bg-green-700 dark:bg-green-500 dark:hover:bg-green-600 text-white rounded-md transition-colors font-medium text-sm"
                    title={contract.balance === 0 ? "Note: You'll need Bitcoin UTXOs to generate an invoice" : "Generate an invoice to receive tokens"}
                  >
                    📨 Receive
                  </button>
                  <button
                    onClick={() => {
                      setSelectedAssetForExport({ contractId: contract.contract_id, ticker: contract.ticker });
                      setShowExportGenesisModal(true);
                    }}
                    className="px-4 py-2 bg-purple-600 hover:bg-purple-700 dark:bg-purple-500 dark:hover:bg-purple-600 text-white rounded-md transition-colors font-medium text-sm"
                  >
                    📦 Export
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {balance && balance.utxos.length > 0 && (
        <UTXOList 
          walletName={name || ''} 
          utxos={balance.utxos} 
          onRefresh={loadWalletData}
        />
      )}

      {balance?.display_address && (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                Your Wallet Address
              </h3>
              <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                Use this address to receive Bitcoin and RGB tokens
              </p>
            </div>
          </div>

          <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 mb-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-gray-600 dark:text-gray-400">
                Public Address
              </span>
              <div className="flex items-center gap-2">
                <a
                  href={`https://mempool.space/signet/address/${balance.display_address}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="px-3 py-1 text-sm font-medium text-gray-700 dark:text-gray-200 hover:text-gray-900 dark:hover:text-white bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md transition-colors inline-flex items-center"
                  title="View on Mempool Explorer"
                >
                  🔗 Explorer
                </a>
                <button
                  onClick={handleCopyAddress}
                  className="px-3 py-1 text-sm font-medium text-primary dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md transition-colors"
                >
                  {addressCopied ? '✓ Copied!' : 'Copy Address'}
                </button>
              </div>
            </div>
            <a
              href={`https://mempool.space/signet/address/${balance.display_address}`}
              target="_blank"
              rel="noopener noreferrer"
              className="block text-sm font-mono bg-white dark:bg-gray-800 p-3 rounded break-all text-primary dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 transition-colors"
              title="Click to view on Mempool Explorer"
            >
              {balance.display_address}
            </a>
          </div>

          <button
            onClick={handleToggleAllAddresses}
            className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 flex items-center"
          >
            {showAllAddresses ? '▼' : '▶'} Advanced: Show all addresses
          </button>

          {showAllAddresses && (
            <div className="mt-4 pt-4 border-t dark:border-gray-700">
              <div className="flex items-center justify-between mb-4">
                <h4 className="text-sm font-semibold text-gray-900 dark:text-white">All Addresses (Advanced)</h4>
                <select
                  value={addressCount}
                  onChange={(e) => { setAddressCount(Number(e.target.value)); loadAllAddresses(); }}
                  className="px-3 py-1 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary"
                >
                  <option value={10}>Show 10</option>
                  <option value={20}>Show 20</option>
                  <option value={50}>Show 50</option>
                </select>
              </div>
              <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">
                ℹ️ Your wallet manages multiple addresses internally for change outputs. Your balance shown above is aggregated from all addresses.
              </p>
              {addresses.length > 0 ? (
                <AddressList addresses={addresses} />
              ) : (
                <div className="text-center py-4">
                  <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-primary mx-auto"></div>
                </div>
              )}
            </div>
          )}
        </div>
      )}

      <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Descriptor</h3>
          <button
            onClick={handleCopyDescriptor}
            className="text-sm text-primary dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300"
          >
            {descriptorCopied ? '✓ Copied' : 'Copy'}
          </button>
        </div>
        <p className="text-xs font-mono bg-gray-50 dark:bg-gray-700 p-3 rounded break-all text-gray-600 dark:text-gray-300">
          {descriptor || 'Loading...'}
        </p>
      </div>

      <CreateUtxoModal
        walletName={name || ''}
        currentBalance={balance?.confirmed_sats || 0}
        isOpen={showCreateUtxoModal}
        onClose={() => setShowCreateUtxoModal(false)}
        onSuccess={() => {
          loadWalletData();
        }}
      />

      <IssueAssetModal
        walletName={name || ''}
        unoccupiedUtxos={balance?.utxos.filter(u => !u.is_occupied) || []}
        isOpen={showIssueAssetModal}
        onClose={() => setShowIssueAssetModal(false)}
        onSuccess={() => {
          setShowIssueAssetModal(false);
          loadWalletData();
        }}
      />

      {selectedAsset && (
        <GenerateInvoiceModal
          walletName={name || ''}
          contractId={selectedAsset.contractId}
          assetTicker={selectedAsset.ticker}
          isOpen={showGenerateInvoiceModal}
          onClose={() => {
            setShowGenerateInvoiceModal(false);
            setSelectedAsset(null);
          }}
        />
      )}

      <SendTransferModal
        walletName={name || ''}
        isOpen={showSendTransferModal}
        onClose={() => setShowSendTransferModal(false)}
        onSyncStart={() => setSyncingRgb(true)}
        onSyncEnd={() => {
          setSyncingRgb(false);
          loadWalletData();
        }}
      />

      <AcceptConsignmentModal
        walletName={name || ''}
        isOpen={showAcceptConsignmentModal}
        onClose={() => setShowAcceptConsignmentModal(false)}
        onSuccess={() => {
          setShowAcceptConsignmentModal(false);
          loadWalletData();
        }}
      />

      {selectedAssetForExport && (
        <ExportGenesisModal
          walletName={name || ''}
          contractId={selectedAssetForExport.contractId}
          assetName={selectedAssetForExport.ticker}
          isOpen={showExportGenesisModal}
          onClose={() => {
            setShowExportGenesisModal(false);
            setSelectedAssetForExport(null);
          }}
        />
      )}

      <SendBitcoinModal
        walletName={name || ''}
        isOpen={showSendBitcoinModal}
        onClose={() => setShowSendBitcoinModal(false)}
        onSuccess={() => {
          setShowSendBitcoinModal(false);
          loadWalletData();
        }}
      />

      <DeleteWalletModal
        walletName={name || ''}
        isOpen={showDeleteWalletModal}
        onClose={() => setShowDeleteWalletModal(false)}
        onSuccess={() => {
          navigate('/');
        }}
      />

      {/* Danger Zone */}
      <div className="mt-8 bg-red-50 dark:bg-red-900/10 border border-red-200 dark:border-red-800 rounded-lg p-6">
        <h3 className="text-lg font-semibold text-red-900 dark:text-red-300 mb-2">
          ⚠️ Danger Zone
        </h3>
        <p className="text-sm text-red-700 dark:text-red-400 mb-4">
          Permanently delete this wallet and all its data. This action cannot be undone.
        </p>
        <button
          onClick={() => setShowDeleteWalletModal(true)}
          className="px-4 py-2 bg-red-600 hover:bg-red-700 dark:bg-red-500 dark:hover:bg-red-600 text-white rounded-md transition-colors font-medium"
        >
          🗑️ Delete Wallet
        </button>
      </div>
    </div>
  );
}
