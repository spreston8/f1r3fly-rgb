import { useState } from 'react';
import { Link } from 'react-router-dom';
import { walletApi } from '../api/wallet';
import type { UTXO, IssueAssetRequest, IssueAssetResponse, IssueAssetResponseWithFirefly } from '../api/types';
import { PRECISION_OPTIONS } from '../api/types';

interface IssueAssetModalProps {
  walletName: string;
  unoccupiedUtxos: UTXO[];
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export default function IssueAssetModal({
  walletName,
  unoccupiedUtxos,
  isOpen,
  onClose,
  onSuccess,
}: IssueAssetModalProps) {
  const [name, setName] = useState('');
  const [ticker, setTicker] = useState('');
  const [precision, setPrecision] = useState(8); // Default to BTC-like precision
  const [supply, setSupply] = useState('');
  const [selectedUtxo, setSelectedUtxo] = useState('');
  const [useFirefly, setUseFirefly] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<IssueAssetResponse | null>(null);
  const [fireflyResult, setFireflyResult] = useState<IssueAssetResponseWithFirefly | null>(null);
  const [copied, setCopied] = useState(false);

  // Validation errors
  const [nameError, setNameError] = useState('');
  const [tickerError, setTickerError] = useState('');
  const [supplyError, setSupplyError] = useState('');
  const [utxoError, setUtxoError] = useState('');

  if (!isOpen) return null;

  const validateForm = (): boolean => {
    let isValid = true;
    setNameError('');
    setTickerError('');
    setSupplyError('');
    setUtxoError('');

    // Validate name (2-12 chars)
    if (name.length < 2 || name.length > 12) {
      setNameError('Name must be 2-12 characters');
      isValid = false;
    }

    // Validate ticker (2-8 chars, uppercase)
    if (ticker.length < 2 || ticker.length > 8) {
      setTickerError('Ticker must be 2-8 characters');
      isValid = false;
    }

    // Validate supply
    const supplyNum = parseInt(supply);
    if (!supply || isNaN(supplyNum) || supplyNum <= 0) {
      setSupplyError('Supply must be a positive number');
      isValid = false;
    }

    // Validate UTXO selection
    if (!selectedUtxo) {
      setUtxoError('Please select a UTXO for the genesis seal');
      isValid = false;
    }

    return isValid;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!validateForm()) {
      return;
    }

    setIsLoading(true);
    setError(null);
    setFireflyResult(null);

    try {
      const request: IssueAssetRequest = {
        name,
        ticker: ticker.toUpperCase(),
        precision,
        supply: parseInt(supply),
        genesis_utxo: selectedUtxo,
      };

      if (useFirefly) {
        // Use F1r3fly/Rholang execution
        const response = await walletApi.issueAssetWithFirefly(walletName, request);
        setFireflyResult(response);
        setSuccess({
          contract_id: response.contract_id,
          genesis_seal: response.genesis_seal,
        });
      } else {
        // Use standard ALuVM execution
        const response = await walletApi.issueAsset(walletName, request);
        setSuccess(response);
      }
    } catch (err: unknown) {
      const error = err as { response?: { data?: { error?: string } }; message?: string };
      setError(error.response?.data?.error || error.message || 'Failed to issue asset');
    } finally {
      setIsLoading(false);
    }
  };

  const handleClose = () => {
    if (!isLoading) {
      // If we're in success state, trigger data refresh before closing
      const wasSuccessful = !!success;
      
      setName('');
      setTicker('');
      setPrecision(8);
      setSupply('');
      setSelectedUtxo('');
      setUseFirefly(false);
      setError(null);
      setSuccess(null);
      setFireflyResult(null);
      setNameError('');
      setTickerError('');
      setSupplyError('');
      setUtxoError('');
      onClose();
      
      // Refresh wallet data after modal closes if issuance was successful
      if (wasSuccessful) {
        onSuccess();
      }
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Success state
  if (success) {
    return (
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 max-w-md w-full mx-4">
          <div className="text-center">
            <div className="text-5xl mb-4">🎉</div>
            <h3 className="text-xl font-bold text-gray-900 dark:text-white mb-2">
              Asset Issued Successfully!
            </h3>
            <p className="text-gray-600 dark:text-gray-400 mb-4">
              Your RGB20 asset has been created on Signet
            </p>
            
            <div className="bg-gray-100 dark:bg-gray-700 rounded-lg p-4 mb-4">
              <div className="text-sm text-gray-600 dark:text-gray-400 mb-2">Contract ID</div>
              <div className="flex items-center justify-between gap-2">
                <code className="text-xs text-gray-800 dark:text-gray-200 break-all">
                  {success.contract_id}
                </code>
                <button
                  onClick={() => copyToClipboard(success.contract_id)}
                  className="flex-shrink-0 text-blue-500 hover:text-blue-600 transition-colors"
                  title="Copy Contract ID"
                >
                  {copied ? '✓' : '📋'}
                </button>
              </div>
            </div>

            <div className="bg-gray-100 dark:bg-gray-700 rounded-lg p-4 mb-4">
              <div className="text-sm text-gray-600 dark:text-gray-400 mb-2">Genesis Seal</div>
              <code className="text-xs text-gray-800 dark:text-gray-200 break-all">
                {success.genesis_seal}
              </code>
            </div>

            {fireflyResult && (
              <div className="bg-gray-100 dark:bg-gray-700 rounded-lg p-4 mb-4">
                <div className="text-sm text-gray-600 dark:text-gray-400 mb-2">F1r3fly</div>
                <div className="space-y-1">
                  <div className="text-xs text-gray-700 dark:text-gray-300">
                    Deploy: <code className="text-gray-800 dark:text-gray-200">{fireflyResult.firefly_deploy_id.substring(0, 16)}...</code>
                  </div>
                  <div className="text-xs text-gray-700 dark:text-gray-300">
                    Block: <code className="text-gray-800 dark:text-gray-200">{fireflyResult.firefly_block_hash.substring(0, 16)}...</code>
                  </div>
                </div>
              </div>
            )}

            <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-md">
              <p className="text-sm text-blue-800 dark:text-blue-300 mb-2">
                📚 Want to understand what just happened?
              </p>
              <Link
                to="/docs/rgb-issuance"
                className="text-sm text-blue-600 dark:text-blue-400 hover:underline font-medium"
                onClick={handleClose}
              >
                Read: Where Will I See My Asset? →
              </Link>
            </div>

            <button
              onClick={handleClose}
              className="w-full px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
            >
              Done
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
            🪙 Issue RGB20 Asset
          </h2>
          <button
            onClick={handleClose}
            disabled={isLoading}
            className="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 text-2xl leading-none disabled:opacity-50"
          >
            ×
          </button>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-red-100 dark:bg-red-900/30 border border-red-300 dark:border-red-700 rounded-md">
            <p className="text-red-700 dark:text-red-400 text-sm">{error}</p>
          </div>
        )}

        {unoccupiedUtxos.length === 0 && (
          <div className="mb-4 p-3 bg-yellow-100 dark:bg-yellow-900/30 border border-yellow-300 dark:border-yellow-700 rounded-md">
            <p className="text-yellow-700 dark:text-yellow-400 text-sm">
              ⚠️ No unoccupied UTXOs available. Please create a UTXO first.
            </p>
          </div>
        )}

        <form onSubmit={handleSubmit}>
          {/* Asset Name */}
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Asset Name <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., MyToken"
              className={`w-full px-3 py-2 border rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                nameError 
                  ? 'border-red-500 dark:border-red-500' 
                  : 'border-gray-300 dark:border-gray-600'
              }`}
              disabled={isLoading}
            />
            {nameError && <p className="mt-1 text-sm text-red-500">{nameError}</p>}
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">2-12 characters</p>
          </div>

          {/* Ticker */}
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Ticker Symbol <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              value={ticker}
              onChange={(e) => setTicker(e.target.value.toUpperCase())}
              placeholder="e.g., MTK"
              className={`w-full px-3 py-2 border rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                tickerError 
                  ? 'border-red-500 dark:border-red-500' 
                  : 'border-gray-300 dark:border-gray-600'
              }`}
              disabled={isLoading}
            />
            {tickerError && <p className="mt-1 text-sm text-red-500">{tickerError}</p>}
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">2-8 characters (auto-uppercased)</p>
          </div>

          {/* Precision */}
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Precision (Decimals) <span className="text-red-500">*</span>
            </label>
            <select
              value={precision}
              onChange={(e) => setPrecision(parseInt(e.target.value))}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
              disabled={isLoading}
            >
              {PRECISION_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label} (e.g., {option.example})
                </option>
              ))}
            </select>
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              Number of decimal places. Choose carefully - this cannot be changed after issuance.
            </p>
          </div>

          {/* Total Supply */}
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Total Supply <span className="text-red-500">*</span>
            </label>
            <input
              type="number"
              value={supply}
              onChange={(e) => setSupply(e.target.value)}
              placeholder="e.g., 1000000"
              min="1"
              step="1"
              className={`w-full px-3 py-2 border rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                supplyError 
                  ? 'border-red-500 dark:border-red-500' 
                  : 'border-gray-300 dark:border-gray-600'
              }`}
              disabled={isLoading}
            />
            {supplyError && <p className="mt-1 text-sm text-red-500">{supplyError}</p>}
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              Fixed supply - cannot mint more later
            </p>
          </div>

          {/* F1r3fly Toggle */}
          <div className="mb-4 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={useFirefly}
                onChange={(e) => setUseFirefly(e.target.checked)}
                className="w-4 h-4 rounded focus:ring-2"
                disabled={isLoading}
              />
              <span className="text-sm text-gray-700 dark:text-gray-300">
                Use F1r3fly/Rholang execution
              </span>
            </label>
          </div>

          {/* Genesis UTXO Selection */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Genesis Seal (UTXO) <span className="text-red-500">*</span>
            </label>
            <select
              value={selectedUtxo}
              onChange={(e) => setSelectedUtxo(e.target.value)}
              className={`w-full px-3 py-2 border rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white ${
                utxoError 
                  ? 'border-red-500 dark:border-red-500' 
                  : 'border-gray-300 dark:border-gray-600'
              }`}
              disabled={isLoading || unoccupiedUtxos.length === 0}
            >
              <option value="">Select a UTXO...</option>
              {unoccupiedUtxos.map((utxo) => (
                <option key={`${utxo.txid}:${utxo.vout}`} value={`${utxo.txid}:${utxo.vout}`}>
                  {utxo.txid.substring(0, 8)}...:{utxo.vout} ({utxo.amount_sats.toLocaleString()} sats)
                </option>
              ))}
            </select>
            {utxoError && <p className="mt-1 text-sm text-red-500">{utxoError}</p>}
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
              This UTXO will become "occupied" with your entire asset supply
            </p>
          </div>

          {/* Buttons */}
          <div className="flex gap-3">
            <button
              type="button"
              onClick={handleClose}
              disabled={isLoading}
              className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-md hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isLoading || unoccupiedUtxos.length === 0}
              className="flex-1 px-4 py-2 bg-green-500 text-white rounded-md hover:bg-green-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isLoading ? 'Issuing...' : '🪙 Issue Asset'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

