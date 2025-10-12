import { useState } from 'react';
import { walletApi } from '../api/wallet';
import type { ExportGenesisResponse } from '../api/types';

interface ExportGenesisModalProps {
  walletName: string;
  contractId: string;
  assetName: string;
  isOpen: boolean;
  onClose: () => void;
}

export default function ExportGenesisModal({
  walletName,
  contractId,
  assetName,
  isOpen,
  onClose,
}: ExportGenesisModalProps) {
  const [isExporting, setIsExporting] = useState(false);
  const [exportResult, setExportResult] = useState<ExportGenesisResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleExport = async () => {
    setError(null);
    setIsExporting(true);

    try {
      const result = await walletApi.exportGenesis(walletName, contractId);
      setExportResult(result);
    } catch (err: unknown) {
      const message = err && typeof err === 'object' && 'response' in err
        ? (err as { response?: { data?: { error?: string } }; message?: string }).response?.data?.error
          || (err as { message?: string }).message
        : 'Export failed';
      setError(message || 'Export failed');
    } finally {
      setIsExporting(false);
    }
  };

  const handleDownload = () => {
    if (exportResult) {
      window.location.href = exportResult.download_url;
    }
  };

  const handleReset = () => {
    setExportResult(null);
    setError(null);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          Export Genesis Consignment
        </h2>

        <div className="mb-4 p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-md">
          <p className="text-sm text-blue-800 dark:text-blue-300">
            <strong>📱 Sync wallet across devices</strong><br />
            Export the genesis consignment and import it on another device with the same wallet mnemonic.
            No Bitcoin transaction is required - this only shares contract knowledge.
          </p>
        </div>

        <div className="mb-4">
          <p className="text-sm text-gray-600 dark:text-gray-400">
            <strong>Asset:</strong> {assetName}<br />
            <strong>Contract ID:</strong> <span className="font-mono text-xs">{contractId}</span>
          </p>
        </div>

        {!exportResult ? (
          <>
            {error && (
              <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 
                            dark:border-red-700 rounded-md text-red-800 dark:text-red-300">
                {error}
              </div>
            )}

            <div className="flex justify-end space-x-3">
              <button
                onClick={onClose}
                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 
                         dark:hover:bg-gray-700 rounded-md transition-colors"
                disabled={isExporting}
              >
                Cancel
              </button>
              <button
                onClick={handleExport}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 dark:bg-blue-500 
                         dark:hover:bg-blue-600 text-white rounded-md transition-colors 
                         disabled:opacity-50"
                disabled={isExporting}
              >
                {isExporting ? 'Exporting...' : '📤 Export Genesis'}
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/30 border border-green-200 
                          dark:border-green-700 rounded-md">
              <h3 className="text-lg font-semibold text-green-800 dark:text-green-300 mb-2">
                ✅ Export Successful
              </h3>
              <p className="text-sm text-green-700 dark:text-green-400">
                Genesis consignment exported. Download and transfer to your other device.
              </p>
            </div>

            <div className="mb-4">
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>File Size:</strong> {(exportResult.file_size_bytes / 1024).toFixed(2)} KB
              </p>
              <p className="text-sm text-gray-600 dark:text-gray-400">
                <strong>Filename:</strong> <span className="font-mono text-xs">{exportResult.consignment_filename}</span>
              </p>
            </div>

            <div className="mb-4 p-4 bg-yellow-50 dark:bg-yellow-900/30 border border-yellow-200 
                          dark:border-yellow-700 rounded-md">
              <p className="text-sm text-yellow-800 dark:text-yellow-300">
                💡 <strong>Next steps:</strong><br />
                1. Download the file<br />
                2. Transfer to your other device (USB/cloud/network)<br />
                3. On the other device: Click "📥 Import Consignment"<br />
                4. The asset will appear (same UTXO, no Bitcoin TX needed)
              </p>
            </div>

            <div className="flex justify-end space-x-3">
              <button
                onClick={handleReset}
                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 
                         dark:hover:bg-gray-700 rounded-md transition-colors"
              >
                Close
              </button>
              <button
                onClick={handleDownload}
                className="px-4 py-2 bg-green-600 hover:bg-green-700 dark:bg-green-500 
                         dark:hover:bg-green-600 text-white rounded-md transition-colors"
              >
                📥 Download File
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

