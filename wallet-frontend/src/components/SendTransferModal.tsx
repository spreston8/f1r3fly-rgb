import { useState } from 'react';
import { walletApi } from '../api/wallet';
import type { SendTransferRequest, SendTransferResponse } from '../api/types';

interface SendTransferModalProps {
  walletName: string;
  isOpen: boolean;
  onClose: () => void;
}

export default function SendTransferModal({
  walletName,
  isOpen,
  onClose,
}: SendTransferModalProps) {
  const [invoice, setInvoice] = useState('');
  const [feeRate, setFeeRate] = useState('1');
  const [result, setResult] = useState<SendTransferResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSend = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsLoading(true);

    try {
      const request: SendTransferRequest = {
        invoice: invoice.trim(),
        fee_rate_sat_vb: parseInt(feeRate) || undefined,
      };

      const response = await walletApi.sendTransfer(walletName, request);
      setResult(response);
    } catch (err: unknown) {
      const message = err && typeof err === 'object' && 'response' in err
        ? (err as { response?: { data?: { error?: string } }; message?: string }).response?.data?.error 
          || (err as { message?: string }).message
        : 'Failed to send transfer';
      setError(message || 'Failed to send transfer');
    } finally {
      setIsLoading(false);
    }
  };

  const handleDownloadConsignment = () => {
    if (result) {
      window.location.href = result.consignment_download_url;
    }
  };

  const handleReset = () => {
    setInvoice('');
    setFeeRate('1');
    setResult(null);
    setError(null);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          Send RGB Transfer
        </h2>

        {!result ? (
          <form onSubmit={handleSend}>
            <div className="mb-4 p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 
                          dark:border-blue-700 rounded-md">
              <p className="text-sm text-blue-800 dark:text-blue-300">
                <strong>How it works:</strong><br />
                1. Paste the invoice you received from the recipient<br />
                2. Set the fee rate (default: 1 sat/vB)<br />
                3. Send the transfer (creates Bitcoin transaction)<br />
                4. Download the consignment file<br />
                5. Share the consignment file with the recipient
              </p>
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                RGB Invoice
              </label>
              <textarea
                value={invoice}
                onChange={(e) => setInvoice(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
                placeholder="Paste invoice string (contract:tb@...)"
                rows={4}
                required
              />
              <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                Invoice format: contract:tb@{'{contract_id}'}/{'{amount}'}@at:{'{auth_token}'}/
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
                placeholder="1"
                min="1"
              />
              <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                Higher fee rate = faster confirmation. Recommended: 1-5 sat/vB for signet.
              </p>
            </div>

            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 
                            dark:border-red-700 rounded-md text-red-800 dark:text-red-300">
                {error}
              </div>
            )}

            <div className="flex justify-end space-x-3">
              <button
                type="button"
                onClick={onClose}
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
                disabled={isLoading || !invoice.trim()}
              >
                {isLoading ? 'Sending...' : 'Send Transfer'}
              </button>
            </div>
          </form>
        ) : (
          <div>
            <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 
                          dark:border-green-700 rounded-md">
              <h3 className="text-lg font-semibold text-green-800 dark:text-green-300 mb-2">
                ✅ Transfer Sent
              </h3>
              <p className="text-sm text-green-700 dark:text-green-400">
                Bitcoin transaction broadcasted successfully. Now download the consignment file
                and share it with the recipient.
              </p>
            </div>

            <div className="mb-4 space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>Bitcoin TX:</strong>{' '}
                <a
                  href={`https://mempool.space/signet/tx/${result.bitcoin_txid}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-600 dark:text-blue-400 hover:underline"
                >
                  {result.bitcoin_txid.slice(0, 16)}...{result.bitcoin_txid.slice(-8)}
                </a>
              </p>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>Status:</strong> {result.status}
              </p>
            </div>

            <div className="mb-4 p-4 bg-yellow-50 dark:bg-yellow-900/30 border border-yellow-200 
                          dark:border-yellow-700 rounded-md">
              <h4 className="text-sm font-semibold text-yellow-800 dark:text-yellow-300 mb-2">
                ⚠️ Important: Share Consignment File
              </h4>
              <p className="text-sm text-yellow-700 dark:text-yellow-400">
                The recipient needs the consignment file to claim their tokens.
                Download it below and share it via email, messaging app, or file transfer.
              </p>
            </div>

            <div className="flex justify-end space-x-3">
              <button
                onClick={handleReset}
                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 
                         dark:hover:bg-gray-700 rounded-md transition-colors"
              >
                Done
              </button>
              <button
                onClick={handleDownloadConsignment}
                className="px-4 py-2 bg-green-600 hover:bg-green-700 dark:bg-green-500 
                         dark:hover:bg-green-600 text-white rounded-md transition-colors"
              >
                📥 Download Consignment
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

