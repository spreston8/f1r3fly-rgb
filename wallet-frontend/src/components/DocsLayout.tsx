import { Link, useLocation } from 'react-router-dom';
import { type ReactNode } from 'react';

interface DocsLayoutProps {
  children: ReactNode;
}

export default function DocsLayout({ children }: DocsLayoutProps) {
  const location = useLocation();

  const navItems = [
    {
      path: '/docs/utxo-creation',
      label: '🎯 RGB UTXO Creation',
      description: 'Understanding RGB seals and UTXO management',
    },
    {
      path: '/docs/rgb-issuance',
      label: '🎬 Asset Issuance',
      description: 'What happens after issuing RGB assets',
    },
    {
      path: '/docs/resources',
      label: '📖 External Resources',
      description: 'Links to RGB protocol documentation',
    },
  ];

  return (
    <div className="max-w-7xl mx-auto">
      <div className="mb-6">
        <Link to="/" className="text-blue-600 dark:text-blue-400 hover:underline">
          ← Back to Wallets
        </Link>
      </div>

      <div className="flex gap-6">
        {/* Left Sidebar */}
        <aside className="w-64 flex-shrink-0">
          <div className="sticky top-6">
            <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4">
              📚 Documentation
            </h2>
            <nav className="space-y-2">
              {navItems.map((item) => {
                const isActive = location.pathname === item.path;
                return (
                  <Link
                    key={item.path}
                    to={item.path}
                    className={`block p-3 rounded-lg transition-colors ${
                      isActive
                        ? 'bg-blue-100 dark:bg-blue-900/30 border-l-4 border-blue-500'
                        : 'bg-white dark:bg-gray-800 border-l-4 border-transparent hover:bg-gray-50 dark:hover:bg-gray-700'
                    }`}
                  >
                    <div className="font-medium text-gray-900 dark:text-white text-sm mb-1">
                      {item.label}
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">
                      {item.description}
                    </div>
                  </Link>
                );
              })}
            </nav>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 min-w-0">
          {children}
        </main>
      </div>
    </div>
  );
}

