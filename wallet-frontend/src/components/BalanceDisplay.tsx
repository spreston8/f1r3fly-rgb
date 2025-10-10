import type { BalanceInfo } from '../api/types';
import { formatSats } from '../utils/format';
import Button from './Button';

interface BalanceDisplayProps {
  balance: BalanceInfo;
  onSync?: () => void;
  syncing?: boolean;
}

export default function BalanceDisplay({ balance, onSync, syncing = false }: BalanceDisplayProps) {
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Balance</h3>
        {onSync && (
          <Button
            onClick={onSync}
            loading={syncing}
            size="sm"
            variant="secondary"
          >
            🔄 Sync
          </Button>
        )}
      </div>

      <div className="space-y-3">
        <div className="flex justify-between items-center">
          <span className="text-sm text-gray-600 dark:text-gray-400">Confirmed:</span>
          <span className="text-lg font-semibold text-gray-900 dark:text-white">
            {formatSats(balance.confirmed_sats)}
          </span>
        </div>

        <div className="flex justify-between items-center">
          <span className="text-sm text-gray-600 dark:text-gray-400">Unconfirmed:</span>
          <span className="text-lg font-semibold text-gray-600 dark:text-gray-300">
            {formatSats(balance.unconfirmed_sats)}
          </span>
        </div>

        <div className="border-t dark:border-gray-700 pt-3 flex justify-between items-center">
          <span className="text-sm font-medium text-gray-900 dark:text-white">Total:</span>
          <span className="text-xl font-bold text-primary dark:text-blue-400">
            {formatSats(balance.confirmed_sats + balance.unconfirmed_sats)}
          </span>
        </div>

        {balance.utxo_count > 0 && (
          <div className="text-sm text-gray-500 dark:text-gray-400 text-center pt-2">
            {balance.utxo_count} UTXO{balance.utxo_count !== 1 ? 's' : ''}
          </div>
        )}
      </div>
    </div>
  );
}

