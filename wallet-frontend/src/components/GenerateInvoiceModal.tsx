import { useState } from 'react';
import { walletApi } from '../api/wallet';
import type { GenerateInvoiceRequest, GenerateInvoiceResponse, UTXO, UtxoSelection } from '../api/types';
import { copyToClipboard } from '../utils/format';

interface GenerateInvoiceModalProps {
  walletName: string;
  contractId: string;    // Pre-filled from asset selection
  assetTicker: string;   // For display
  availableUtxos: UTXO[]; // Available UTXOs for selection
  isOpen: boolean;
  onClose: () => void;
}

export default function GenerateInvoiceModal({
  walletName,
  contractId,
  assetTicker,
  availableUtxos,
  isOpen,
  onClose,
}: GenerateInvoiceModalProps) {
  const [amount, setAmount] = useState('');
  const [invoice, setInvoice] = useState<GenerateInvoiceResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [utxoSelectionMode, setUtxoSelectionMode] = useState<'auto' | 'specific'>('specific'); // Default to 'specific' (safer)
  const [selectedUtxo, setSelectedUtxo] = useState<string>(''); // Format: "txid:vout"

  const handleGenerate = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsLoading(true);

    try {
      // Build UTXO selection
      let utxo_selection: UtxoSelection | undefined;
      if (utxoSelectionMode === 'specific') {
        if (!selectedUtxo) {
          setError('Please select a UTXO');
          setIsLoading(false);
          return;
        }
        const [txid, voutStr] = selectedUtxo.split(':');
        utxo_selection = { type: 'specific', txid, vout: parseInt(voutStr) };
      } else {
        utxo_selection = { type: 'auto' };
      }

      const request: GenerateInvoiceRequest = {
        contract_id: contractId,
        amount: parseInt(amount),
        utxo_selection,
      };

      const response = await walletApi.generateInvoice(walletName, request);
      setInvoice(response);
    } catch (err: unknown) {
      const errorMsg = (err as { response?: { data?: { error?: string } }; message?: string })?.response?.data?.error 
        || (err as { message?: string })?.message 
        || 'Failed to generate invoice';
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
    setUtxoSelectionMode('specific'); // Reset to safer default
    setSelectedUtxo('');
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

            {/* UTXO Selection */}
            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Invoice Type
              </label>
              
              {/* Selection Mode Tabs */}
              <div className="flex gap-2 mb-3">
                <button
                  type="button"
                  onClick={() => setUtxoSelectionMode('specific')}
                  className={`flex-1 px-4 py-2 rounded-md font-medium text-sm transition-colors ${
                    utxoSelectionMode === 'specific'
                      ? 'bg-green-600 dark:bg-green-500 text-white'
                      : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'
                  }`}
                >
                  ✅ Isolated (Recommended)
                </button>
                <button
                  type="button"
                  onClick={() => setUtxoSelectionMode('auto')}
                  className={`flex-1 px-4 py-2 rounded-md font-medium text-sm transition-colors ${
                    utxoSelectionMode === 'auto'
                      ? 'bg-yellow-600 dark:bg-yellow-500 text-white'
                      : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'
                  }`}
                >
                  ⚠️ Automatic (Advanced)
                </button>
              </div>

              {/* Isolated Mode Explanation */}
              {utxoSelectionMode === 'specific' && (
                <>
                  <div className="p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-700 rounded-md mb-3">
                    <p className="text-sm font-semibold text-green-800 dark:text-green-300 mb-1">
                      ✅ Isolated Mode (Recommended)
                    </p>
                    <p className="text-xs text-green-700 dark:text-green-400 leading-relaxed">
                      Sender will create a <strong>new 5000-sat UTXO</strong> at your address to receive tokens.
                      This keeps RGB tokens <strong>separate from your main Bitcoin holdings</strong>, preventing lockup.
                    </p>
                    <div className="mt-2 text-xs text-green-600 dark:text-green-500">
                      <strong>✓</strong> Your existing UTXOs stay clean<br/>
                      <strong>✓</strong> No risk of locking high-value Bitcoin<br/>
                      <strong>✓</strong> Easy to manage RGB tokens separately<br/>
                      <strong>-</strong> Costs sender ~5000 sats (~$0.02)
                    </div>
                  </div>
                  <select
                    value={selectedUtxo}
                    onChange={(e) => setSelectedUtxo(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                             bg-white dark:bg-gray-700 text-gray-900 dark:text-white mb-2"
                    required={utxoSelectionMode === 'specific'}
                  >
                    <option value="">-- Select Address (via UTXO) --</option>
                    {availableUtxos.map((utxo) => (
                      <option key={`${utxo.txid}:${utxo.vout}`} value={`${utxo.txid}:${utxo.vout}`}>
                        {utxo.address} ({utxo.amount_sats.toLocaleString()} sats)
                        {utxo.is_occupied && ' [OCCUPIED]'}
                      </option>
                    ))}
                  </select>
                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    💡 Selecting a UTXO here only identifies the address. A new UTXO will be created there.
                  </p>
                </>
              )}

              {/* Auto Mode Explanation */}
              {utxoSelectionMode === 'auto' && (
                <div className="p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700 rounded-md">
                  <p className="text-sm font-semibold text-yellow-800 dark:text-yellow-300 mb-1">
                    ⚠️ Automatic Mode (Advanced Users)
                  </p>
                  <p className="text-xs text-yellow-700 dark:text-yellow-400 leading-relaxed mb-2">
                    RGB runtime will automatically select one of your <strong>existing UTXOs</strong> to receive tokens.
                    This provides better privacy but may lock up Bitcoin.
                  </p>
                  <div className="text-xs text-yellow-600 dark:text-yellow-500">
                    <strong>✓</strong> Higher privacy (blinded invoice)<br/>
                    <strong>✓</strong> No extra cost to sender<br/>
                    <strong>⚠️</strong> May pick a <strong>high-value UTXO</strong> (unpredictable)<br/>
                    <strong>⚠️</strong> Selected UTXO will be "locked" with RGB tokens<br/>
                    <strong>⚠️</strong> Must transfer RGB tokens before spending that Bitcoin
                  </div>
                  <p className="text-xs text-yellow-800 dark:text-yellow-300 font-semibold mt-2">
                    ⚠️ Use only if you understand the risks
                  </p>
                </div>
              )}
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
                
                {/* Show selected UTXO details if available (Isolated mode) */}
                {invoice.selected_utxo && (
                  <>
                    <div className="mt-3 pt-3 border-t border-gray-300 dark:border-gray-600">
                      <p className="text-xs font-semibold text-green-700 dark:text-green-300 mb-2">
                        ✅ Isolated Mode - New UTXO Will Be Created
                      </p>
                      <p className="text-xs text-gray-600 dark:text-gray-400 mb-2">
                        When the sender pays this invoice, they will create a <strong>new 5000-sat UTXO</strong> at the address below.
                        Your RGB tokens will be received at this new UTXO, keeping them isolated from your existing Bitcoin.
                      </p>
                      <p className="text-xs">
                        <strong>Address:</strong>{' '}
                        <span className="font-mono text-xs text-blue-600 dark:text-blue-400">{invoice.selected_utxo.address}</span>
                      </p>
                      <p className="text-xs text-gray-500 dark:text-gray-500 mt-2">
                        <em>Reference UTXO used to identify address:</em>
                      </p>
                      <p className="text-xs">
                        <strong>TXID:</strong>{' '}
                        <a
                          href={`https://mempool.space/signet/tx/${invoice.selected_utxo.txid}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="font-mono text-blue-600 dark:text-blue-400 hover:underline"
                        >
                          {invoice.selected_utxo.txid.slice(0, 16)}...{invoice.selected_utxo.txid.slice(-8)}
                        </a>
                      </p>
                      <p className="text-xs">
                        <strong>Vout:</strong> {invoice.selected_utxo.vout}
                      </p>
                    </div>
                  </>
                )}
                
                {/* Show auto-selection message if no specific UTXO (Automatic mode) */}
                {!invoice.selected_utxo && (
                  <div className="mt-3 pt-3 border-t border-gray-300 dark:border-gray-600">
                    <p className="text-xs font-semibold text-yellow-700 dark:text-yellow-300 mb-1">
                      ⚠️ Automatic Mode - Existing UTXO Will Be Used
                    </p>
                    <p className="text-xs text-yellow-600 dark:text-yellow-400">
                      RGB runtime will automatically select one of your existing UTXOs to receive tokens.
                      The selected UTXO will be "locked" and cannot be spent without first transferring the RGB tokens.
                    </p>
                  </div>
                )}
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

