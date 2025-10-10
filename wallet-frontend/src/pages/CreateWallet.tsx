import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { walletApi } from '../api';
import type { WalletInfo } from '../api/types';
import Button from '../components/Button';
import { copyToClipboard } from '../utils/format';

export default function CreateWallet() {
  const navigate = useNavigate();
  const [name, setName] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [walletInfo, setWalletInfo] = useState<WalletInfo | null>(null);
  const [copied, setCopied] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError('Please enter a wallet name');
      return;
    }

    try {
      setLoading(true);
      setError(null);
      const info = await walletApi.createWallet(name.trim());
      setWalletInfo(info);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create wallet');
    } finally {
      setLoading(false);
    }
  };

  const handleCopyMnemonic = async () => {
    if (walletInfo) {
      const success = await copyToClipboard(walletInfo.mnemonic);
      if (success) {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }
    }
  };

  const handleGoToWallet = () => {
    if (walletInfo) {
      navigate(`/wallet/${walletInfo.name}`);
    }
  };

  if (walletInfo) {
    return (
      <div className="max-w-2xl mx-auto space-y-6">
        <div className="text-center">
          <div className="inline-flex items-center justify-center w-16 h-16 bg-green-100 dark:bg-green-900/30 rounded-full mb-4">
            <svg className="w-8 h-8 text-green-600 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
          </div>
          <h2 className="text-3xl font-bold text-gray-900 dark:text-white">Wallet Created!</h2>
        </div>

        <div className="bg-yellow-50 dark:bg-yellow-900/20 border-2 border-yellow-400 dark:border-yellow-600 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-yellow-900 dark:text-yellow-300 mb-2">
            🔑 Recovery Phrase (12 words)
          </h3>
          <p className="text-sm text-yellow-800 dark:text-yellow-400 mb-4">
            Save this phrase securely! You'll need it to recover your wallet.
          </p>
          <div className="bg-white dark:bg-gray-800 rounded p-4 font-mono text-sm break-words text-gray-900 dark:text-gray-100">
            {walletInfo.mnemonic}
          </div>
          <div className="mt-4">
            <Button onClick={handleCopyMnemonic} variant="secondary" className="w-full">
              {copied ? '✓ Copied!' : 'Copy to Clipboard'}
            </Button>
          </div>
        </div>

        <div className="bg-white dark:bg-gray-800 shadow dark:shadow-gray-900 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">First Address</h3>
          <p className="text-sm font-mono bg-gray-50 dark:bg-gray-700 p-3 rounded break-all text-gray-900 dark:text-gray-100">
            {walletInfo.first_address}
          </p>
        </div>

        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
          <p className="text-sm text-red-800 dark:text-red-300 font-medium">
            ⚠️ IMPORTANT: Save your recovery phrase before continuing!
          </p>
          <p className="text-sm text-red-700 dark:text-red-400 mt-1">
            Without it, you cannot recover your wallet if you lose access.
          </p>
        </div>

        <Button onClick={handleGoToWallet} className="w-full">
          Go to Wallet →
        </Button>
      </div>
    );
  }

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div className="flex items-center space-x-4">
        <Link to="/" className="text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200">
          ← Back
        </Link>
        <h2 className="text-3xl font-bold text-gray-900 dark:text-white">Create New Wallet</h2>
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

          {error && (
            <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3">
              <p className="text-sm text-red-800 dark:text-red-300">{error}</p>
            </div>
          )}

          <Button type="submit" loading={loading} className="w-full">
            Create Wallet
          </Button>
        </form>

        <div className="mt-6 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded p-4">
          <p className="text-sm text-blue-800 dark:text-blue-300">
            ℹ️ After creation, you'll receive a 12-word recovery phrase. Save it securely!
          </p>
        </div>
      </div>
    </div>
  );
}
