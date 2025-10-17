import { useState } from 'react';

interface CreateUtxoModalProps {
  walletName: string;
  currentBalance: number;
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export default function CreateUtxoModal({
  walletName,
  currentBalance,
  isOpen,
  onClose,
  onSuccess,
}: CreateUtxoModalProps) {
  const [mode, setMode] = useState<'default' | 'custom'>('default');
  const [customAmount, setCustomAmount] = useState('0.0003');
  const [customFeeRate, setCustomFeeRate] = useState('2');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      const body = mode === 'default' 
        ? {} 
        : {
            amount_btc: parseFloat(customAmount),
            fee_rate_sat_vb: parseInt(customFeeRate),
          };

      const response = await fetch(`/api/wallet/${walletName}/create-utxo`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || 'Failed to create UTXO');
      }

      onSuccess();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create UTXO');
    } finally {
      setLoading(false);
    }
  };

  const defaultAmountBTC = 0.0003;
  const currentBalanceBTC = currentBalance / 100_000_000;

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl dark:shadow-gray-900 max-w-md w-full mx-4">
        <div className="p-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-xl font-bold text-gray-900 dark:text-white">
              Create RGB UTXO
            </h3>
            <button
              onClick={onClose}
              className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
            Create a Bitcoin UTXO at your wallet address for RGB operations.
          </p>

          <div className="flex border-b border-gray-200 dark:border-gray-700 mb-4">
            <button
              onClick={() => setMode('default')}
              className={`px-4 py-2 text-sm font-medium ${
                mode === 'default'
                  ? 'border-b-2 border-primary text-primary dark:text-blue-400'
                  : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
              }`}
            >
              Default
            </button>
            <button
              onClick={() => setMode('custom')}
              className={`px-4 py-2 text-sm font-medium ${
                mode === 'custom'
                  ? 'border-b-2 border-primary text-primary dark:text-blue-400'
                  : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
              }`}
            >
              Custom
            </button>
          </div>

          <form onSubmit={handleSubmit}>
            {mode === 'default' ? (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    The UTXO creation amount
                  </label>
                  <div className="text-2xl font-bold text-gray-900 dark:text-white">
                    {defaultAmountBTC} BTC
                  </div>
                  <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                    (~30,000 sats)
                  </p>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    Balances:
                  </label>
                  <div className="text-lg text-gray-900 dark:text-white">
                    {currentBalanceBTC.toFixed(8)} BTC
                  </div>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    Fee
                  </label>
                  <div className="text-gray-900 dark:text-white">2 sat/vB</div>
                </div>
              </div>
            ) : (
              <div className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    Available BTC
                  </label>
                  <div className="text-lg text-gray-900 dark:text-white mb-3">
                    {currentBalanceBTC.toFixed(8)} BTC
                  </div>
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Amount (BTC)
                  </label>
                  <input
                    type="number"
                    step="0.00000001"
                    min="0.0001"
                    max={currentBalanceBTC}
                    value={customAmount}
                    onChange={(e) => setCustomAmount(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary dark:focus:ring-blue-500 focus:border-transparent"
                    required
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Fee Rate (sat/vB)
                  </label>
                  <input
                    type="number"
                    min="1"
                    value={customFeeRate}
                    onChange={(e) => setCustomFeeRate(e.target.value)}
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary dark:focus:ring-blue-500 focus:border-transparent"
                    required
                  />
                </div>
              </div>
            )}

            {error && (
              <div className="mt-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
                <p className="text-sm text-red-800 dark:text-red-300">{error}</p>
              </div>
            )}

            <div className="flex space-x-3 mt-6">
              <button
                type="button"
                onClick={onClose}
                className="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-md text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                disabled={loading}
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={loading}
                className="flex-1 px-4 py-2 bg-primary hover:bg-blue-600 dark:bg-blue-500 dark:hover:bg-blue-600 text-white rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {loading ? 'Creating...' : 'Create UTXO'}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}

