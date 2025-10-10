import { useState } from 'react';
import type { AddressInfo } from '../api/types';
import { copyToClipboard } from '../utils/format';

interface AddressListProps {
  addresses: AddressInfo[];
}

export default function AddressList({ addresses }: AddressListProps) {
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  const handleCopy = async (address: string, index: number) => {
    const success = await copyToClipboard(address);
    if (success) {
      setCopiedIndex(index);
      setTimeout(() => setCopiedIndex(null), 2000);
    }
  };

  if (addresses.length === 0) {
    return (
      <div className="text-center py-8 text-gray-500 dark:text-gray-400">
        No addresses found
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {addresses.map((addr) => (
        <div
          key={addr.index}
          className="flex items-center justify-between bg-gray-50 dark:bg-gray-700 rounded-lg p-3 hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors"
        >
          <div className="flex items-center space-x-3 flex-1 min-w-0">
            <span className="text-sm font-mono text-gray-500 dark:text-gray-400 flex-shrink-0">
              #{addr.index}
            </span>
            <span className="text-sm font-mono text-gray-900 dark:text-gray-100 truncate">
              {addr.address}
            </span>
            {addr.used && (
              <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300">
                Used
              </span>
            )}
          </div>
          <button
            onClick={() => handleCopy(addr.address, addr.index)}
            className="ml-2 p-2 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 flex-shrink-0"
            title="Copy address"
          >
            {copiedIndex === addr.index ? (
              <svg className="w-5 h-5 text-green-500 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
            ) : (
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
              </svg>
            )}
          </button>
        </div>
      ))}
    </div>
  );
}

