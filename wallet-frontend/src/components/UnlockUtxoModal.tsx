import { useState } from 'react';
import type { UTXO } from '../api/types';
import { walletApi } from '../api/wallet';
import { formatSats, truncateHash, copyToClipboard } from '../utils/format';

interface UnlockUtxoModalProps {
  walletName: string;
  utxo: UTXO | null;
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export default function UnlockUtxoModal({
  walletName,
  utxo,
  isOpen,
  onClose,
  onSuccess,
}: UnlockUtxoModalProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [copiedContractId, setCopiedContractId] = useState<string | null>(null);

  const handleCopyContractId = async (contractId: string) => {
    const success = await copyToClipboard(contractId);
    if (success) {
      setCopiedContractId(contractId);
      setTimeout(() => setCopiedContractId(null), 2000);
    }
  };

  const handleUnlock = async () => {
    if (!utxo) return;
    
    if (utxo.is_occupied && !confirmed) {
      setError('Please confirm that you understand you will lose these RGB assets');
      return;
    }

    setLoading(true);
    setError(null);

    try {
      await walletApi.unlockUtxo(walletName, {
        txid: utxo.txid,
        vout: utxo.vout,
      });

      onSuccess();
    } catch (err: any) {
      setError(err.response?.data?.error || err.message || 'Failed to unlock UTXO');
      setLoading(false);
    }
  };

  const resetForm = () => {
    setError(null);
    setConfirmed(false);
    setLoading(false);
  };

  if (!isOpen || !utxo) return null;

  const estimatedFee = 200; // Rough estimate for display
  const estimatedRecovery = utxo.amount_sats - estimatedFee;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            {utxo.is_occupied ? '⚠️ Unlock UTXO' : '🔓 Unlock UTXO'}
          </h2>
        </div>

        <div className="p-6 space-y-4">
          {/* UTXO Details */}
          <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Output:</span>
                <span className="text-sm font-mono text-gray-900 dark:text-white">
                  {truncateHash(utxo.txid, 12)}:{utxo.vout}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Amount:</span>
                <span className="text-sm font-semibold text-gray-900 dark:text-white">
                  {formatSats(utxo.amount_sats)}
                </span>
              </div>
            </div>
          </div>

          {/* Notice */}
          <div className="bg-blue-50 dark:bg-blue-950 border border-blue-200 dark:border-blue-900 rounded-lg p-4">
            <p className="text-sm text-blue-900 dark:text-blue-200">
              <strong>Notice:</strong> UTXO unlocking requires a transaction fee. After unlocking,
              the available BTC in the original UTXO will be transferred to your wallet balance.
            </p>
          </div>

          {/* RGB Asset Warning - Only for occupied UTXOs */}
          {utxo.is_occupied && utxo.bound_assets && utxo.bound_assets.length > 0 && (
            <div className="bg-orange-50 dark:bg-orange-950/50 border border-orange-300 dark:border-orange-800 rounded-lg p-4 space-y-3">
              <div className="flex items-start space-x-3">
                <span className="text-xl">⚠️</span>
                <div className="flex-1">
                  <p className="text-sm font-semibold text-orange-900 dark:text-orange-200 mb-3">
                    This UTXO contains RGB assets that will be forfeited:
                  </p>
                  <div className="space-y-2">
                    {utxo.bound_assets.map((asset, idx) => (
                      <div
                        key={`${asset.asset_id}-${idx}`}
                        className="bg-white dark:bg-gray-800 rounded p-3 border border-gray-200 dark:border-gray-700"
                      >
                        <div className="flex items-start justify-between mb-2">
                          <div className="flex items-center space-x-2">
                            <span className="inline-flex items-center px-2 py-1 rounded text-xs font-bold bg-orange-200 dark:bg-orange-900 text-orange-900 dark:text-orange-200">
                              {asset.ticker}
                            </span>
                            <span className="text-sm font-medium text-gray-900 dark:text-white">
                              {asset.asset_name}
                            </span>
                          </div>
                        </div>
                        <div className="text-xs text-gray-600 dark:text-gray-400 mb-1">
                          Amount: <span className="font-mono font-semibold">{asset.amount}</span>
                        </div>
                        <div className="flex items-center space-x-1">
                          <span className="text-xs text-gray-500 dark:text-gray-500">
                            Contract: {asset.asset_id}
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
                    ))}
                  </div>
                  <p className="text-sm text-orange-800 dark:text-orange-300 mt-3">
                    Note: These assets cannot be recovered after unlocking.
                  </p>
                </div>
              </div>

              {/* Confirmation Checkbox */}
              <label className="flex items-start space-x-3 cursor-pointer">
                <input
                  type="checkbox"
                  checked={confirmed}
                  onChange={(e) => setConfirmed(e.target.checked)}
                  className="mt-1 w-4 h-4 text-orange-600 border-gray-300 rounded focus:ring-orange-500"
                />
                <span className="text-sm text-gray-700 dark:text-gray-300">
                  I understand these assets will be forfeited
                </span>
              </label>
            </div>
          )}

          {/* Fee Estimate */}
          <div className="text-sm text-gray-600 dark:text-gray-400">
            <p>Estimated fee: ~{estimatedFee} sats</p>
            <p>Estimated recovery: ~{formatSats(estimatedRecovery)}</p>
          </div>

          {/* Error Display */}
          {error && (
            <div className="bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-900 rounded-lg p-3">
              <p className="text-sm text-red-900 dark:text-red-200">{error}</p>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end space-x-3">
          <button
            onClick={() => {
              resetForm();
              onClose();
            }}
            disabled={loading}
            className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-md transition-colors disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            onClick={handleUnlock}
            disabled={loading || (utxo.is_occupied && !confirmed)}
            className={`px-4 py-2 text-sm font-medium text-white rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed ${
              utxo.is_occupied
                ? 'bg-orange-600 hover:bg-orange-700 dark:bg-orange-700 dark:hover:bg-orange-800'
                : 'bg-blue-500 hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700'
            }`}
          >
            {loading ? 'Unlocking...' : utxo.is_occupied ? 'Unlock UTXO' : 'Unlock UTXO'}
          </button>
        </div>
      </div>
    </div>
  );
}

