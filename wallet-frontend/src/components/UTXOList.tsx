import { useState } from 'react';
import type { UTXO } from '../api/types';
import { formatSats, truncateHash, copyToClipboard } from '../utils/format';

interface UTXOListProps {
  utxos: UTXO[];
}

export default function UTXOList({ utxos }: UTXOListProps) {
  const [copiedTxid, setCopiedTxid] = useState<string | null>(null);

  const handleCopy = async (txid: string) => {
    const success = await copyToClipboard(txid);
    if (success) {
      setCopiedTxid(txid);
      setTimeout(() => setCopiedTxid(null), 2000);
    }
  };

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
        UTXOs ({utxos.length})
      </h3>
      
      <div className="space-y-3">
        {utxos.map((utxo) => (
          <div
            key={`${utxo.txid}:${utxo.vout}`}
            className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1 min-w-0">
                <div className="flex items-center space-x-2 mb-2">
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
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

