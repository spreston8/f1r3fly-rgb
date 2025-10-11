import DocsLayout from '../components/DocsLayout';

export default function DocsResources() {
  return (
    <DocsLayout>
      <div className="space-y-6">
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-8">
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-6">
            📖 External Resources
          </h1>

          <p className="text-gray-700 dark:text-gray-300 mb-8">
            Explore these external resources to learn more about the RGB protocol, Bitcoin Signet, and related tools.
          </p>

          {/* RGB Protocol */}
          <section className="mb-8">
            <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
              RGB Protocol
            </h2>
            <div className="space-y-4">
              <a
                href="https://rgb.tech"
                target="_blank"
                rel="noopener noreferrer"
                className="block p-4 bg-gray-50 dark:bg-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors border border-gray-200 dark:border-gray-600"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-1">
                      RGB Official Website
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      Learn about RGB smart contracts on Bitcoin and Lightning Network
                    </p>
                  </div>
                  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </div>
              </a>

              <a
                href="https://github.com/RGB-WG"
                target="_blank"
                rel="noopener noreferrer"
                className="block p-4 bg-gray-50 dark:bg-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors border border-gray-200 dark:border-gray-600"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-1">
                      RGB Working Group GitHub
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      Explore the open-source RGB protocol implementation
                    </p>
                  </div>
                  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </div>
              </a>

              <a
                href="https://docs.rgb.info"
                target="_blank"
                rel="noopener noreferrer"
                className="block p-4 bg-gray-50 dark:bg-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors border border-gray-200 dark:border-gray-600"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-1">
                      RGB Documentation
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      Technical documentation and developer guides
                    </p>
                  </div>
                  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </div>
              </a>
            </div>
          </section>

          {/* Bitcoin Signet */}
          <section className="mb-8">
            <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
              Bitcoin Signet
            </h2>
            <div className="space-y-4">
              <a
                href="https://mempool.space/signet"
                target="_blank"
                rel="noopener noreferrer"
                className="block p-4 bg-gray-50 dark:bg-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors border border-gray-200 dark:border-gray-600"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-1">
                      Signet Mempool Explorer
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      View transactions, addresses, and blocks on Bitcoin Signet
                    </p>
                  </div>
                  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </div>
              </a>

              <a
                href="https://en.bitcoin.it/wiki/Signet"
                target="_blank"
                rel="noopener noreferrer"
                className="block p-4 bg-gray-50 dark:bg-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors border border-gray-200 dark:border-gray-600"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-1">
                      Bitcoin Wiki: Signet
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      Learn about Bitcoin's Signet test network
                    </p>
                  </div>
                  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </div>
              </a>
            </div>
          </section>

          {/* Development Tools */}
          <section>
            <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
              Development Tools
            </h2>
            <div className="space-y-4">
              <a
                href="https://github.com/BP-WG"
                target="_blank"
                rel="noopener noreferrer"
                className="block p-4 bg-gray-50 dark:bg-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors border border-gray-200 dark:border-gray-600"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-1">
                      Bitcoin Protocol Working Group
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      Standards and libraries for Bitcoin protocol development
                    </p>
                  </div>
                  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </div>
              </a>

              <a
                href="https://github.com/strict-types"
                target="_blank"
                rel="noopener noreferrer"
                className="block p-4 bg-gray-50 dark:bg-gray-700 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-600 transition-colors border border-gray-200 dark:border-gray-600"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-1">
                      Strict Types
                    </h3>
                    <p className="text-sm text-gray-600 dark:text-gray-400">
                      Type system used by RGB for contract validation
                    </p>
                  </div>
                  <svg className="w-5 h-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                  </svg>
                </div>
              </a>
            </div>
          </section>
        </div>
      </div>
    </DocsLayout>
  );
}

