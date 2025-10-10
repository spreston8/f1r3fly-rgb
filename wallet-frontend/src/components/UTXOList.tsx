import { useState } from 'react';
import type { UTXO } from '../api/types';
import { formatSats, truncateHash, copyToClipboard } from '../utils/format';
import UnlockUtxoModal from './UnlockUtxoModal';

interface UTXOListProps {
  walletName: string;
  utxos: UTXO[];
  onRefresh?: () => void;
}

type TabType = 'unoccupied' | 'occupied';

export default function UTXOList({ walletName, utxos, onRefresh }: UTXOListProps) {
  const unoccupiedCount = utxos.filter((u) => !u.is_occupied).length;
  const occupiedCount = utxos.filter((u) => u.is_occupied).length;
  
  // Smart default: Default to "occupied" if any exist, otherwise "unoccupied"
  const defaultTab: TabType = occupiedCount > 0 ? 'occupied' : 'unoccupied';
  
  const [activeTab, setActiveTab] = useState<TabType>(defaultTab);
  const [copiedTxid, setCopiedTxid] = useState<string | null>(null);
  const [copiedContractId, setCopiedContractId] = useState<string | null>(null);
  const [selectedUtxo, setSelectedUtxo] = useState<UTXO | null>(null);
  const [showUnlockModal, setShowUnlockModal] = useState(false);

  const handleCopy = async (txid: string) => {
    const success = await copyToClipboard(txid);
    if (success) {
      setCopiedTxid(txid);
      setTimeout(() => setCopiedTxid(null), 2000);
    }
  };

  const handleCopyContractId = async (contractId: string) => {
    const success = await copyToClipboard(contractId);
    if (success) {
      setCopiedContractId(contractId);
      setTimeout(() => setCopiedContractId(null), 2000);
    }
  };

  const handleUnlock = (utxo: UTXO) => {
    setSelectedUtxo(utxo);
    setShowUnlockModal(true);
  };

  const handleUnlockSuccess = () => {
    setShowUnlockModal(false);
    setSelectedUtxo(null);
    if (onRefresh) {
      onRefresh();
    }
  };

  const filteredUtxos = utxos.filter((utxo) => {
    if (activeTab === 'unoccupied') return !utxo.is_occupied;
    if (activeTab === 'occupied') return utxo.is_occupied;
    return true;
  });

  if (utxos.length === 0) {
    return (
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">UTXOs</h3>
        <div className="text-center py-8 text-gray-500 dark:text-gray-400">
          No UTXOs found
        </div>
      </div>
    );
  }

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
        UTXOs
      </h3>
      
      {/* Tab Navigation */}
      <div className="flex space-x-1 mb-4 border-b border-gray-200 dark:border-gray-700">
        <button
          onClick={() => setActiveTab('unoccupied')}
          className={`px-4 py-2 text-sm font-medium transition-colors ${
            activeTab === 'unoccupied'
              ? 'text-primary dark:text-blue-400 border-b-2 border-primary dark:border-blue-400'
              : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
          }`}
        >
          Unoccupied ({unoccupiedCount})
        </button>
        <button
          onClick={() => setActiveTab('occupied')}
          className={`px-4 py-2 text-sm font-medium transition-colors ${
            activeTab === 'occupied'
              ? 'text-primary dark:text-blue-400 border-b-2 border-primary dark:border-blue-400'
              : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
          }`}
        >
          Occupied ({occupiedCount})
        </button>
      </div>

      {/* Empty State */}
      {filteredUtxos.length === 0 ? (
        <div className="text-center py-8">
          {activeTab === 'unoccupied' && (
            <div className="text-gray-500 dark:text-gray-400">
              No unoccupied UTXOs
            </div>
          )}
          {activeTab === 'occupied' && (
            <div>
              <div className="text-gray-500 dark:text-gray-400 mb-4">
                No occupied UTXOs
              </div>
              <div className="max-w-md mx-auto bg-blue-50 dark:bg-blue-950 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
                <div className="flex items-start space-x-3">
                  <span className="text-xl">ℹ️</span>
                  <div className="flex-1 text-left">
                    <h4 className="text-sm font-semibold text-blue-900 dark:text-blue-200 mb-1">
                      About Occupied UTXOs
                    </h4>
                    <p className="text-sm text-blue-800 dark:text-blue-300">
                      RGB assets bind to specific UTXOs, making them "occupied." When a UTXO is occupied, 
                      you can unlock it to recover the Bitcoin (forfeiting the RGB assets).
                    </p>
                    <p className="text-xs text-blue-700 dark:text-blue-400 mt-2">
                      💡 Occupied UTXOs will appear here after you issue RGB assets.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      ) : (
        <div className="space-y-3">
          {filteredUtxos.map((utxo) => (
          <div
            key={`${utxo.txid}:${utxo.vout}`}
            className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                <div className="flex items-center space-x-2 mb-2">
                  {/* Status Badge */}
                  {utxo.is_occupied ? (
                    <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-orange-100 dark:bg-orange-900 text-orange-800 dark:text-orange-200">
                      🔒 RGB Asset
                    </span>
                  ) : (
                    <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200">
                      ✓ Available
                    </span>
                  )}
                  <a
                    href={`https://mempool.space/signet/tx/${utxo.txid}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm font-mono text-primary dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 transition-colors"
                    title="View transaction on Mempool Explorer"
                  >
                    {truncateHash(utxo.txid, 12)}:{utxo.vout}
                  </a>
                  <button
                    onClick={() => handleCopy(utxo.txid)}
                    className="text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
                    title="Copy transaction ID"
                  >
                    {copiedTxid === utxo.txid ? (
                      <svg className="w-4 h-4 text-green-500 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                      </svg>
                    ) : (
                      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                      </svg>
                    )}
                  </button>
                  <a
                    href={`https://mempool.space/signet/tx/${utxo.txid}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200 transition-colors"
                    title="View on Mempool Explorer"
                  >
                    🔗 Explorer
                  </a>
                </div>
                
                <div className="flex items-center space-x-4 text-sm text-gray-600 dark:text-gray-400">
                  <span className="font-semibold text-gray-900 dark:text-white">
                    {formatSats(utxo.amount_sats)}
                  </span>
                  <span>
                    {utxo.confirmations} confirmation{utxo.confirmations !== 1 ? 's' : ''}
                  </span>
                </div>

                {/* Bound RGB Assets Section */}
                {utxo.is_occupied && utxo.bound_assets && utxo.bound_assets.length > 0 && (
                  <div className="mt-4 pt-3 border-t border-gray-200 dark:border-gray-600">
                    <div className="text-xs font-semibold text-gray-700 dark:text-gray-300 mb-2">
                      Bound RGB Assets:
                    </div>
                    <div className="space-y-2">
                      {utxo.bound_assets.map((asset, idx) => (
                        <div
                          key={`${asset.asset_id}-${idx}`}
                          className="bg-white dark:bg-gray-800 rounded p-2 border border-orange-200 dark:border-orange-800"
                        >
                          <div className="flex items-start justify-between">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center space-x-2 mb-1">
                                <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-bold bg-orange-200 dark:bg-orange-900 text-orange-900 dark:text-orange-200">
                                  {asset.ticker}
                                </span>
                                <span className="text-sm font-medium text-gray-900 dark:text-white">
                                  {asset.asset_name}
                                </span>
                              </div>
                              <div className="flex items-center space-x-2">
                                <span className="text-xs text-gray-600 dark:text-gray-400">
                                  Amount: <span className="font-mono font-semibold">{asset.amount}</span>
                                </span>
                              </div>
                              <div className="flex items-center space-x-1 mt-1">
                                <span className="text-xs text-gray-500 dark:text-gray-500">
                                  Contract: {truncateHash(asset.asset_id, 8)}
                                </span>
                                <button
                                  onClick={() => handleCopyContractId(asset.asset_id)}
                                  className="text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300"
                                  title="Copy contract ID"
                                >
                                  {copiedContractId === asset.asset_id ? (
                                    <svg className="w-3 h-3 text-green-500 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                                    </svg>
                                  ) : (
                                    <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                                    </svg>
                                  )}
                                </button>
                              </div>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>

              {/* Unlock Button - Only show on Occupied tab */}
              {activeTab === 'occupied' && utxo.is_occupied && (
                <div className="ml-4 flex-shrink-0">
                  <button
                    onClick={() => handleUnlock(utxo)}
                    className="px-3 py-2 rounded-md font-medium text-sm transition-colors bg-red-600 hover:bg-red-700 dark:bg-red-700 dark:hover:bg-red-800 text-white"
                    title="Unlock UTXO (will forfeit RGB assets)"
                  >
                    ⚠️ Unlock
                  </button>
                </div>
              )}
            </div>
          </div>
        ))}
        </div>
      )}

      {/* Unlock UTXO Modal */}
      <UnlockUtxoModal
        walletName={walletName}
        utxo={selectedUtxo}
        isOpen={showUnlockModal}
        onClose={() => {
          setShowUnlockModal(false);
          setSelectedUtxo(null);
        }}
        onSuccess={handleUnlockSuccess}
      />
    </div>
  );
}

