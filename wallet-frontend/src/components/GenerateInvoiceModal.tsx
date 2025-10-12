import { useState } from 'react';
import { walletApi } from '../api/wallet';
import type { GenerateInvoiceRequest, GenerateInvoiceResponse } from '../api/types';
import { copyToClipboard } from '../utils/format';

interface GenerateInvoiceModalProps {
  walletName: string;
  contractId: string;    // Pre-filled from asset selection
  assetTicker: string;   // For display
  isOpen: boolean;
  onClose: () => void;
}

export default function GenerateInvoiceModal({
  walletName,
  contractId,
  assetTicker,
  isOpen,
  onClose,
}: GenerateInvoiceModalProps) {
  const [amount, setAmount] = useState('');
  const [invoice, setInvoice] = useState<GenerateInvoiceResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const handleGenerate = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsLoading(true);

    try {
      const request: GenerateInvoiceRequest = {
        contract_id: contractId,
        amount: parseInt(amount),
      };

      const response = await walletApi.generateInvoice(walletName, request);
      setInvoice(response);
    } catch (err: any) {
      const errorMsg = err.response?.data?.error || err.message || 'Failed to generate invoice';
      // Add helpful context for timeout errors
      if (errorMsg.includes('timeout')) {
        setError('Request timed out. The RGB runtime sync may be taking longer than expected. Please try again.');
      } else {
        setError(errorMsg);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleCopy = async () => {
    if (invoice) {
      await copyToClipboard(invoice.invoice);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleClose = () => {
    // Reset state when closing
    setAmount('');
    setInvoice(null);
    setError(null);
    setCopied(false);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          📨 Generate Invoice
        </h2>

        {!invoice ? (
          <form onSubmit={handleGenerate}>
            <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-md">
              <p className="text-sm text-blue-800 dark:text-blue-300">
                <strong>Asset:</strong> {assetTicker}
              </p>
              <p className="text-xs text-blue-600 dark:text-blue-400 mt-1">
                Contract ID: {contractId}
              </p>
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Amount to Receive *
              </label>
              <input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                placeholder="Enter amount"
                required
                min="1"
              />
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Specify how many tokens you want to receive
              </p>
            </div>

            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 
                            dark:border-red-700 rounded-md text-red-800 dark:text-red-300">
                {error}
              </div>
            )}

            {isLoading && (
              <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-md">
                <div className="flex items-center space-x-3">
                  <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600 dark:border-blue-400"></div>
                  <div className="flex-1">
                    <p className="text-sm font-medium text-blue-800 dark:text-blue-300">
                      Generating invoice...
                    </p>
                    <p className="text-xs text-blue-600 dark:text-blue-400 mt-1">
                      Syncing RGB runtime with blockchain (first time may take 30-60 seconds)
                    </p>
                  </div>
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
                {isLoading ? 'Generating...' : 'Generate Invoice'}
              </button>
            </div>
          </form>
        ) : (
          <div>
            <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 
                          dark:border-green-700 rounded-md">
              <h3 className="text-lg font-semibold text-green-800 dark:text-green-300 mb-2">
                ✅ Invoice Generated Successfully
              </h3>
              <p className="text-sm text-green-700 dark:text-green-400">
                Share this invoice with the sender to receive your {assetTicker} tokens
              </p>
            </div>

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Invoice String
              </label>
              <div className="relative">
                <textarea
                  value={invoice.invoice}
                  readOnly
                  rows={4}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                           bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white font-mono text-sm"
                />
                <button
                  onClick={handleCopy}
                  className="absolute top-2 right-2 px-3 py-1 bg-blue-600 hover:bg-blue-700 
                           dark:bg-blue-500 dark:hover:bg-blue-600 text-white text-sm rounded 
                           transition-colors"
                >
                  {copied ? '✓ Copied' : '📋 Copy'}
                </button>
              </div>
            </div>

            <div className="mb-4 p-3 bg-gray-50 dark:bg-gray-700 rounded-md border border-gray-200 dark:border-gray-600">
              <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">
                Invoice Details
              </h4>
              <div className="text-sm text-gray-600 dark:text-gray-400 space-y-1">
                <p><strong>Amount:</strong> {invoice.amount} {assetTicker}</p>
                <p><strong>Seal UTXO:</strong> <span className="font-mono text-xs">{invoice.seal_utxo}</span></p>
                <p><strong>Contract:</strong> <span className="font-mono text-xs">{invoice.contract_id}</span></p>
              </div>
            </div>

            <div className="mb-4 p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700 rounded-md">
              <p className="text-sm text-yellow-800 dark:text-yellow-300">
                <strong>⚠️ Important:</strong> After the sender processes this invoice, they will provide you with a 
                <strong> consignment file</strong>. You'll need to upload that file to complete the transfer and receive your tokens.
              </p>
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

