import { useState, useEffect } from 'react';
import { Link, useParams } from 'react-router-dom';
import { walletApi } from '../api';
import type { BalanceInfo, AddressInfo, NextAddressInfo } from '../api/types';
import BalanceDisplay from '../components/BalanceDisplay';
import AddressList from '../components/AddressList';
import UTXOList from '../components/UTXOList';
import CreateUtxoModal from '../components/CreateUtxoModal';
import { copyToClipboard } from '../utils/format';

export default function WalletDetail() {
  const { name } = useParams<{ name: string }>();
  const [balance, setBalance] = useState<BalanceInfo | null>(null);
  const [nextAddress, setNextAddress] = useState<NextAddressInfo | null>(null);
  const [addresses, setAddresses] = useState<AddressInfo[]>([]);
  const [showAllAddresses, setShowAllAddresses] = useState(false);
  const [addressCount, setAddressCount] = useState(20);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [addressCopied, setAddressCopied] = useState(false);
  const [descriptorCopied, setDescriptorCopied] = useState(false);
  const [showCreateUtxoModal, setShowCreateUtxoModal] = useState(false);

  useEffect(() => {
    if (name) {
      loadWalletData();
    }
  }, [name]);

  const loadWalletData = async () => {
    if (!name) return;

    try {
      setLoading(true);
      setError(null);
      
      const [balanceData, nextAddressData] = await Promise.all([
        walletApi.getBalance(name),
        walletApi.getPrimaryAddress(name),
      ]);
      
      setBalance(balanceData);
      setNextAddress(nextAddressData);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load wallet data');
    } finally {
      setLoading(false);
    }
  };

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
    if (nextAddress) {
      const success = await copyToClipboard(nextAddress.address);
      if (success) {
        setAddressCopied(true);
        setTimeout(() => setAddressCopied(false), 2000);
      }
    }
  };

  const handleCopyDescriptor = async () => {
    if (nextAddress) {
      const success = await copyToClipboard(nextAddress.descriptor);
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
        <button
          onClick={() => setShowCreateUtxoModal(true)}
          className="px-4 py-2 bg-green-600 hover:bg-green-700 dark:bg-green-500 dark:hover:bg-green-600 text-white rounded-md transition-colors font-medium"
        >
          ➕ Create UTXO
        </button>
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

      {balance && balance.utxos.length > 0 && (
        <UTXOList utxos={balance.utxos} />
      )}

      {nextAddress && (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                Primary Receive Address
              </h3>
              <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                Use this address for all deposits • {nextAddress.total_used > 0 ? `${nextAddress.total_used} address${nextAddress.total_used !== 1 ? 'es' : ''} have received funds` : 'No deposits yet'}
              </p>
            </div>
          </div>

          <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 mb-4">
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-gray-600 dark:text-gray-400">
                Address #{nextAddress.index}
              </span>
              <div className="flex items-center gap-2">
                <a
                  href={`https://mempool.space/signet/address/${nextAddress.address}`}
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
              href={`https://mempool.space/signet/address/${nextAddress.address}`}
              target="_blank"
              rel="noopener noreferrer"
              className="block text-sm font-mono bg-white dark:bg-gray-800 p-3 rounded break-all text-primary dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 transition-colors"
              title="Click to view on Mempool Explorer"
            >
              {nextAddress.address}
            </a>
          </div>

          <button
            onClick={handleToggleAllAddresses}
            className="text-sm text-primary dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 flex items-center"
          >
            {showAllAddresses ? '▼' : '▶'} Show all addresses
          </button>

          {showAllAddresses && (
            <div className="mt-4 pt-4 border-t dark:border-gray-700">
              <div className="flex items-center justify-between mb-4">
                <h4 className="text-sm font-semibold text-gray-900 dark:text-white">All Addresses</h4>
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
          {nextAddress ? nextAddress.descriptor : 'Loading...'}
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
    </div>
  );
}
