import { useState } from 'react';
import { walletApi } from '../api/wallet';
import type { SendBitcoinRequest, SendBitcoinResponse } from '../api/types';

interface SendBitcoinModalProps {
  walletName: string;
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export default function SendBitcoinModal({
  walletName,
  isOpen,
  onClose,
  onSuccess,
}: SendBitcoinModalProps) {
  const [toAddress, setToAddress] = useState('');
  const [amountSats, setAmountSats] = useState('');
  const [feeRate, setFeeRate] = useState('2');
  const [result, setResult] = useState<SendBitcoinResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsLoading(true);

    try {
      const request: SendBitcoinRequest = {
        to_address: toAddress,
        amount_sats: parseInt(amountSats),
        fee_rate_sat_vb: parseInt(feeRate),
      };

      const response = await walletApi.sendBitcoin(walletName, request);
      setResult(response);
      onSuccess();
    } catch (err: unknown) {
      const message = err && typeof err === 'object' && 'response' in err
        ? (err as { response?: { data?: { error?: string } }; message?: string }).response?.data?.error
          || (err as { message?: string }).message
        : 'Failed to send Bitcoin';
      setError(message || 'Failed to send Bitcoin');
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    setToAddress('');
    setAmountSats('');
    setFeeRate('2');
    setResult(null);
    setError(null);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          💸 Send Bitcoin
        </h2>

        {!result ? (
          <form onSubmit={handleSend}>
            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Recipient Address *
              </label>
              <input
                type="text"
                value={toAddress}
                onChange={(e) => setToAddress(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
                placeholder="tb1q..."
                required
              />
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Amount (sats) *
              </label>
              <input
                type="number"
                value={amountSats}
                onChange={(e) => setAmountSats(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                placeholder="10000"
                required
                min="546"
              />
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Minimum: 546 sats (dust limit)
              </p>
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Fee Rate (sat/vB)
              </label>
              <input
                type="number"
                value={feeRate}
                onChange={(e) => setFeeRate(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                placeholder="2"
                min="1"
              />
            </div>

            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 
                            dark:border-red-700 rounded-md">
                <div className="text-red-800 dark:text-red-300 whitespace-pre-line">
                  {error}
                </div>
              </div>
            )}

            {isLoading && (
              <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-md">
                <div className="flex items-center space-x-3">
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600 dark:border-blue-400"></div>
                  <p className="text-sm font-medium text-blue-800 dark:text-blue-300">
                    Sending Bitcoin...
                  </p>
                </div>
              </div>
            )}

            <div className="flex justify-end space-x-3">
              <button
                type="button"
                onClick={handleClose}
                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 
                         dark:hover:bg-gray-700 rounded-md transition-colors"
                disabled={isLoading}
              >
                Cancel
              </button>
              <button
                type="submit"
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 
                         dark:hover:bg-blue-600 text-white rounded-md transition-colors 
                         disabled:opacity-50"
                disabled={isLoading}
              >
                {isLoading ? 'Sending...' : 'Send Bitcoin'}
              </button>
            </div>
          </form>
        ) : (
          <div>
            <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 
                          dark:border-green-700 rounded-md">
              <h3 className="text-lg font-semibold text-green-800 dark:text-green-300 mb-2">
                ✅ Bitcoin Sent Successfully
              </h3>
              <p className="text-sm text-green-700 dark:text-green-400">
                Your Bitcoin has been broadcast to the network
              </p>
            </div>

            <div className="mb-4 p-3 bg-gray-50 dark:bg-gray-700 rounded-md border border-gray-200 dark:border-gray-600">
              <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">
                Transaction Details
              </h4>
              <div className="text-sm text-gray-600 dark:text-gray-400 space-y-1">
                <p><strong>Amount:</strong> {result.amount_sats.toLocaleString()} sats</p>
                <p><strong>Fee:</strong> {result.fee_sats.toLocaleString()} sats</p>
                <p><strong>To:</strong> <span className="font-mono text-xs break-all">{result.to_address}</span></p>
                <p>
                  <strong>TX ID:</strong>{' '}
                  <a
                    href={`https://mempool.space/signet/tx/${result.txid}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 dark:text-blue-400 hover:underline font-mono text-xs break-all"
                  >
                    {result.txid}
                  </a>
                </p>
              </div>
            </div>

            <div className="flex justify-end">
              <button
                onClick={handleClose}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-700 dark:bg-gray-500 
                         dark:hover:bg-gray-600 text-white rounded-md transition-colors"
              >
                Done
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

