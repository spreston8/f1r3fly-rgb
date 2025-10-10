import { Link } from 'react-router-dom';
import type { WalletMetadata } from '../api/types';
import { formatDate } from '../utils/format';

interface WalletCardProps {
  wallet: WalletMetadata;
}

export default function WalletCard({ wallet }: WalletCardProps) {
  return (
    <Link
      to={`/wallet/${wallet.name}`}
      className="block bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 hover:shadow-md dark:hover:shadow-gray-700 transition-shadow p-6"
    >
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
            {wallet.name}
          </h3>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Created: {formatDate(wallet.created_at)}
          </p>
          {wallet.last_synced ? (
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {wallet.last_synced}
            </p>
          ) : (
            <p className="text-sm text-gray-400 dark:text-gray-500">
              Not synced yet
            </p>
          )}
        </div>
        <div className="text-gray-400 dark:text-gray-500">
          <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
          </svg>
        </div>
      </div>
    </Link>
  );
}

