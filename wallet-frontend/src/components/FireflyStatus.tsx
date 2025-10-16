import { useEffect, useState } from 'react';
import { walletApi } from '../api/wallet';
import type { FireflyNodeStatus } from '../api/types';

export function FireflyStatus() {
  const [status, setStatus] = useState<FireflyNodeStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStatus = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await walletApi.getFireflyStatus();
      setStatus(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch F1r3fly status');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStatus();
    // Refresh every 5 seconds
    const interval = setInterval(fetchStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  if (loading && !status) {
    return (
      <div className="flex items-center gap-2 px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg">
        <div className="w-2 h-2 rounded-full bg-gray-400 dark:bg-gray-500"></div>
        <span className="text-xs text-gray-600 dark:text-gray-400">F1r3fly</span>
      </div>
    );
  }

  if (error) {
    return (
      <button
        onClick={fetchStatus}
        className="flex items-center gap-2 px-3 py-1.5 bg-red-50 dark:bg-red-900/20 hover:bg-red-100 dark:hover:bg-red-900/30 rounded-lg transition-colors"
        title={error}
      >
        <div className="w-2 h-2 rounded-full bg-red-500"></div>
        <span className="text-xs text-red-700 dark:text-red-400">F1r3fly</span>
      </button>
    );
  }

  if (!status) return null;

  const isConnected = status.node_connected;
  const bgClass = isConnected 
    ? 'bg-green-50 dark:bg-green-900/20 hover:bg-green-100 dark:hover:bg-green-900/30' 
    : 'bg-red-50 dark:bg-red-900/20 hover:bg-red-100 dark:hover:bg-red-900/30';
  const textClass = isConnected 
    ? 'text-green-700 dark:text-green-400' 
    : 'text-red-700 dark:text-red-400';
  const dotClass = isConnected ? 'bg-green-500' : 'bg-red-500';

  return (
    <button
      onClick={fetchStatus}
      disabled={loading}
      className={`flex items-center gap-2 px-3 py-1.5 ${bgClass} rounded-lg transition-colors disabled:opacity-50`}
      title={`F1r3fly Node: ${status.node_url}`}
    >
      <div className={`w-2 h-2 rounded-full ${dotClass}`}></div>
      <span className={`text-xs ${textClass} font-medium`}>F1r3fly</span>
    </button>
  );
}

