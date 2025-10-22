import { useState } from 'react';
import { walletApi } from '../api/wallet';
import type { AcceptConsignmentResponse } from '../api/types';

interface AcceptConsignmentModalProps {
  walletName: string;
  isOpen: boolean;
  importType: 'transfer' | 'genesis' | null;
  onClose: () => void;
  onSuccess: () => void;
}

export default function AcceptConsignmentModal({
  walletName,
  isOpen,
  importType,
  onClose,
  onSuccess,
}: AcceptConsignmentModalProps) {
  const [file, setFile] = useState<File | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [result, setResult] = useState<AcceptConsignmentResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      setFile(e.target.files[0]);
      setError(null);
    }
  };

  const handleImport = async () => {
    if (!file) return;

    setError(null);
    setIsImporting(true);

    try {
      const arrayBuffer = await file.arrayBuffer();
      const bytes = new Uint8Array(arrayBuffer);
      
      const importResult = await walletApi.acceptConsignment(walletName, bytes);
      setResult(importResult);
      
      // Refresh wallet data after successful import
      setTimeout(() => {
        onSuccess();
      }, 2000);
    } catch (err: unknown) {
      const message = err && typeof err === 'object' && 'response' in err
        ? (err as { response?: { data?: { error?: string } }; message?: string }).response?.data?.error 
          || (err as { message?: string }).message
        : 'Import failed';
      setError(message || 'Import failed');
    } finally {
      setIsImporting(false);
    }
  };

  const handleReset = () => {
    setFile(null);
    setResult(null);
    setError(null);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          {importType === 'transfer' ? '📥 Accept Transfer' :
           importType === 'genesis' ? '📦 Import RGB Asset' :
           '📥 Import RGB Asset'}
        </h2>

        {!result ? (
          <>
            {/* Conditional explanation based on importType */}
            {importType === 'transfer' && (
              <div className="mb-6 space-y-4">
                <div className="p-4 bg-purple-50 dark:bg-purple-900/30 border border-purple-200 dark:border-purple-700 rounded-md">
                  <h3 className="text-sm font-bold text-purple-900 dark:text-purple-200 mb-2">
                    🔄 Accept RGB Transfer
                  </h3>
                  <p className="text-xs text-purple-800 dark:text-purple-300">
                    Upload the <strong>transfer file</strong> you received to claim your tokens. 
                    The sender should have given you a file named <code className="text-xs bg-purple-200 dark:bg-purple-800 px-1 rounded">transfer_*.rgbc</code>
                  </p>
                </div>

                <div className="p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-md">
                  <h4 className="text-xs font-bold text-blue-900 dark:text-blue-200 mb-2">
                    ℹ️ What happens after you accept:
                  </h4>
                  <ol className="text-xs text-blue-800 dark:text-blue-300 space-y-1 ml-4 list-decimal">
                    <li>We'll verify the cryptographic proofs</li>
                    <li>We'll check the Bitcoin transaction status</li>
                    <li>Your balance will update once the Bitcoin TX confirms (1+ blocks)</li>
                    <li>The asset will appear in your RGB Assets list</li>
                  </ol>
                </div>
              </div>
            )}

            {importType === 'genesis' && (
              <div className="mb-6 space-y-4">
                <div className="p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-700 rounded-md">
                  <h3 className="text-sm font-bold text-green-900 dark:text-green-200 mb-2">
                    📱 Import RGB Asset (Genesis)
                  </h3>
                  <p className="text-xs text-green-800 dark:text-green-300">
                    Upload a <strong>genesis file</strong> to learn about an RGB contract. 
                    The file should be named <code className="text-xs bg-green-200 dark:bg-green-800 px-1 rounded">genesis_*.rgbc</code>
                  </p>
                </div>

                <div className="p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-md">
                  <h4 className="text-xs font-bold text-blue-900 dark:text-blue-200 mb-2">
                    ℹ️ What happens after you import:
                  </h4>
                  <ul className="text-xs text-blue-800 dark:text-blue-300 space-y-1 ml-4 list-disc">
                    <li>The contract will appear in your RGB Assets list</li>
                    <li>Your balance will be <strong>0</strong> (you don't own tokens yet)</li>
                    <li>You'll be able to generate invoices to receive tokens</li>
                    <li>Two use cases:
                      <ul className="ml-4 mt-1 space-y-1">
                        <li>• Syncing your own contract to a new device (same wallet)</li>
                        <li>• Learning about a token someone else created (different wallet)</li>
                      </ul>
                    </li>
                  </ul>
                </div>
              </div>
            )}

            {!importType && (
              <div className="mb-6 p-4 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md">
                <p className="text-sm text-gray-800 dark:text-gray-300">
                  <strong>Import a consignment file:</strong><br />
                  RGB uses consignment files (.rgbc) to share contract information and token ownership. 
                  Upload the file you received to import the asset.
                </p>
              </div>
            )}

            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                {importType === 'transfer' ? 'Select Transfer File (.rgbc)' :
                 importType === 'genesis' ? 'Select Genesis File (.rgbc)' :
                 'Select Consignment File (.rgbc)'}
              </label>
              <input
                type="file"
                accept=".rgbc,.rgb,.consignment"
                onChange={handleFileSelect}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md 
                         bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
              />
              {file && (
                <div className="mt-2">
                  <p className="text-sm text-gray-600 dark:text-gray-400">
                    Selected: {file.name} ({(file.size / 1024).toFixed(2)} KB)
                  </p>
                  {/* Validation warning */}
                  {importType === 'transfer' && !file.name.startsWith('transfer_') && (
                    <p className="text-xs text-yellow-600 dark:text-yellow-400 mt-1 flex items-center gap-1">
                      ⚠️ This doesn't look like a transfer file. Transfer files usually start with "transfer_"
                    </p>
                  )}
                  {importType === 'genesis' && !file.name.startsWith('genesis_') && (
                    <p className="text-xs text-yellow-600 dark:text-yellow-400 mt-1 flex items-center gap-1">
                      ⚠️ This doesn't look like a genesis file. Genesis files usually start with "genesis_"
                    </p>
                  )}
                </div>
              )}
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
                disabled={isImporting}
              >
                Cancel
              </button>
              <button
                onClick={handleImport}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 
                         dark:hover:bg-blue-600 text-white rounded-md transition-colors 
                         disabled:opacity-50"
                disabled={!file || isImporting}
              >
                {isImporting ? (
                  importType === 'transfer' ? 'Accepting Transfer...' : 'Importing Asset...'
                ) : (
                  importType === 'transfer' ? 'Accept Transfer' : 'Import Asset'
                )}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className={`mb-4 p-4 rounded-md border ${
              result.import_type === 'transfer'
                ? 'bg-purple-50 dark:bg-purple-900/30 border-purple-200 dark:border-purple-700'
                : 'bg-green-50 dark:bg-green-900/30 border-green-200 dark:border-green-700'
            }`}>
              <h3 className={`text-lg font-semibold mb-2 ${
                result.import_type === 'transfer'
                  ? 'text-purple-800 dark:text-purple-300'
                  : 'text-green-800 dark:text-green-300'
              }`}>
                {result.import_type === 'transfer' ? '✅ Transfer Accepted!' : '✅ Asset Imported!'}
              </h3>
              <p className={`text-sm ${
                result.import_type === 'transfer'
                  ? 'text-purple-700 dark:text-purple-400'
                  : 'text-green-700 dark:text-green-400'
              }`}>
                {result.import_type === 'transfer'
                  ? 'Your tokens will appear after the Bitcoin transaction confirms (1+ blocks).'
                  : 'The contract is now in your wallet. Generate an invoice to receive tokens.'}
              </p>
            </div>

            <div className="mb-4 space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>Contract ID:</strong> <span className="font-mono text-xs">{result.contract_id}</span>
              </p>
              
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>Type:</strong>{' '}
                <span className={`px-2 py-1 rounded text-xs font-semibold ${
                  result.import_type === 'genesis'
                    ? 'bg-blue-100 dark:bg-blue-800 text-blue-800 dark:text-blue-200'
                    : result.import_type === 'transfer'
                    ? 'bg-purple-100 dark:bg-purple-800 text-purple-800 dark:text-purple-200'
                    : 'bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200'
                }`}>
                  {result.import_type === 'genesis' ? '🎁 Genesis' :
                   result.import_type === 'transfer' ? '💸 Transfer' :
                   result.import_type}
                </span>
              </p>
              
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>Status:</strong>{' '}
                <span className={`px-2 py-1 rounded text-xs font-semibold ${
                  result.status === 'confirmed'
                    ? 'bg-green-100 dark:bg-green-800 text-green-800 dark:text-green-200'
                    : result.status === 'pending'
                    ? 'bg-yellow-100 dark:bg-yellow-800 text-yellow-800 dark:text-yellow-200'
                    : result.status === 'genesis_imported'
                    ? 'bg-blue-100 dark:bg-blue-800 text-blue-800 dark:text-blue-200'
                    : 'bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-200'
                }`}>
                  {result.status === 'confirmed' ? '✅ Confirmed' :
                   result.status === 'pending' ? '⏳ Pending' :
                   result.status === 'genesis_imported' ? '🎁 Genesis' :
                   result.status}
                </span>
              </p>
              
              {result.bitcoin_txid && (
                <p className="text-sm text-gray-600 dark:text-gray-400">
                  <strong>Bitcoin TX:</strong>{' '}
                  <a
                    href={`https://mempool.space/signet/tx/${result.bitcoin_txid}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 dark:text-blue-400 hover:underline font-mono text-xs"
                  >
                    {result.bitcoin_txid.slice(0, 16)}...{result.bitcoin_txid.slice(-8)}
                  </a>
                </p>
              )}
            </div>

            <div className="mb-4 p-4 bg-yellow-50 dark:bg-yellow-900/30 border border-yellow-200 
                          dark:border-yellow-700 rounded-md">
              <p className="text-sm text-yellow-800 dark:text-yellow-300">
                💡 <strong>Tip:</strong> Click "Sync Wallet" to update your balance and see the imported assets.
              </p>
            </div>

            <div className="flex justify-end">
              <button
                onClick={handleReset}
                className="px-4 py-2 bg-gray-600 hover:bg-gray-700 dark:bg-gray-500 
                         dark:hover:bg-gray-600 text-white rounded-md transition-colors"
              >
                Done
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

