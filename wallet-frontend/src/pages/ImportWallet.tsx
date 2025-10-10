import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { walletApi } from '../api';
import Button from '../components/Button';

export default function ImportWallet() {
  const navigate = useNavigate();
  const [name, setName] = useState('');
  const [mnemonic, setMnemonic] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const validateMnemonic = (mnemonic: string): boolean => {
    const words = mnemonic.trim().split(/\s+/);
    return words.length === 12;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!name.trim()) {
      setError('Please enter a wallet name');
      return;
    }

    if (!validateMnemonic(mnemonic)) {
      setError('Please enter a valid 12-word recovery phrase');
      return;
    }

    try {
      setLoading(true);
      setError(null);
      const info = await walletApi.importWallet(name.trim(), mnemonic.trim());
      navigate(`/wallet/${info.name}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to import wallet');
    } finally {
      setLoading(false);
    }
  };

  const wordCount = mnemonic.trim() ? mnemonic.trim().split(/\s+/).length : 0;

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div className="flex items-center space-x-4">
        <Link to="/" className="text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200">
          ← Back
        </Link>
        <h2 className="text-3xl font-bold text-gray-900 dark:text-white">Import Wallet</h2>
      </div>

      <div className="bg-white dark:bg-gray-800 shadow dark:shadow-gray-900 rounded-lg p-6">
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label htmlFor="name" className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Wallet Name
            </label>
            <input
              type="text"
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder="Enter wallet name"
              disabled={loading}
            />
          </div>

          <div>
            <label htmlFor="mnemonic" className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Recovery Phrase (12 words)
            </label>
            <textarea
              id="mnemonic"
              value={mnemonic}
              onChange={(e) => setMnemonic(e.target.value)}
              rows={4}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white rounded-md focus:outline-none focus:ring-2 focus:ring-primary font-mono text-sm"
              placeholder="word1 word2 word3 ..."
              disabled={loading}
            />
            <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
              Word count: {wordCount}/12
            </p>
          </div>

          {error && (
            <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3">
              <p className="text-sm text-red-800 dark:text-red-300">{error}</p>
            </div>
          )}

          <Button type="submit" loading={loading} className="w-full">
            Import Wallet
          </Button>
        </form>

        <div className="mt-6 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700 rounded p-4">
          <p className="text-sm text-yellow-800 dark:text-yellow-300">
            ⚠️ Enter the 12-word recovery phrase from your existing wallet. Make sure it's correct!
          </p>
        </div>
      </div>
    </div>
  );
}
