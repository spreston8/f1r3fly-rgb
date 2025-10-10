import { Link } from 'react-router-dom';

export default function Docs() {
  return (
    <div className="max-w-4xl mx-auto space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
          📚 Documentation
        </h1>
        <Link
          to="/"
          className="text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
        >
          ← Back to Wallets
        </Link>
      </div>

      {/* RGB UTXO Creation Section */}
      <section className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          🎯 Understanding RGB UTXO Creation
        </h2>

        <div className="space-y-6 text-gray-700 dark:text-gray-300">
          <div>
            <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
              What is an RGB UTXO?
            </h3>
            <p>
              RGB uses <strong>UTXOs as "seals"</strong> for assets. Think of each UTXO as a container 
              that can hold RGB tokens. When you issue RGB tokens, they get bound to a specific UTXO.
            </p>
          </div>

          <div className="bg-blue-50 dark:bg-blue-900/20 border-l-4 border-blue-500 p-4">
            <p className="font-mono text-sm">
              Issue 1000 MyTokens<br />
              ↓<br />
              Bind to UTXO: 35d86d6a...a8db:0 (30,000 sats)<br />
              ↓<br />
              This UTXO now holds: 30,000 sats BTC + 1000 MyTokens RGB
            </p>
          </div>

          <div>
            <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
              Why Create Separate RGB UTXOs?
            </h3>
            <p className="mb-4">
              The key insight is about <strong>UTXO amount management</strong>, not address separation.
            </p>

            <div className="space-y-4">
              <div>
                <h4 className="font-semibold text-gray-900 dark:text-white mb-2">
                  ❌ Without Pre-Creating UTXOs:
                </h4>
                <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3 font-mono text-sm">
                  Your wallet:<br />
                  └─ UTXO: 1,100,030 sats<br />
                  <br />
                  You issue 1000 RGB tokens:<br />
                  └─ UTXO: 1,100,030 sats + 1000 tokens (OCCUPIED)<br />
                  <br />
                  <span className="text-red-600 dark:text-red-400 font-bold">
                    Problem: ALL your Bitcoin is now locked with RGB tokens!<br />
                    You can't spend Bitcoin without dealing with RGB assets.
                  </span>
                </div>
              </div>

              <div>
                <h4 className="font-semibold text-gray-900 dark:text-white mb-2">
                  ✅ With Pre-Created UTXOs:
                </h4>
                <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded p-3 font-mono text-sm">
                  Your wallet:<br />
                  ├─ UTXO: 30,000 sats (RGB-ready)<br />
                  └─ UTXO: 1,069,738 sats (Free spending money)<br />
                  <br />
                  You issue 1000 RGB tokens:<br />
                  ├─ UTXO: 30,000 sats + 1000 tokens (OCCUPIED)<br />
                  └─ UTXO: 1,069,738 sats (Still free!)<br />
                  <br />
                  <span className="text-green-600 dark:text-green-400 font-bold">
                    ✅ You can spend the 1.069M sats without touching RGB<br />
                    ✅ RGB tokens are isolated to the 30k UTXO
                  </span>
                </div>
              </div>
            </div>
          </div>

          <div>
            <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
              Why 30,000 Sats?
            </h3>
            <p className="mb-2">The 30k sats serves two purposes:</p>
            <ul className="list-disc list-inside space-y-2 ml-4">
              <li>
                <strong>Transaction Fees:</strong> When you transfer RGB tokens, you need to pay 
                Bitcoin transaction fees. 30k sats (~$30 if BTC is $100k) ensures you can always 
                pay fees without running out.
              </li>
              <li>
                <strong>RGB Change:</strong> RGB transfers work like Bitcoin transfers. You need 
                multiple UTXOs to handle change when sending partial amounts.
              </li>
            </ul>
          </div>

          <div>
            <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
              Why Same Address is Fine
            </h3>
            <p className="mb-2">
              <strong>RGB tracks by UTXO outpoint, NOT address.</strong>
            </p>
            <p className="mb-4">
              RGB identifies UTXOs by <code className="bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded">txid:vout</code>, 
              not by address:
            </p>

            <div className="bg-gray-50 dark:bg-gray-700 rounded p-3 font-mono text-sm mb-4">
              Example:<br />
              - 35d86d6a...a8db:0 (the 30k UTXO)<br />
              - 35d86d6a...a8db:1 (the 1.069M UTXO)<br />
              <br />
              Same transaction, different vout → Different UTXOs<br />
              <span className="text-blue-600 dark:text-blue-400 font-bold">
                RGB can tell them apart!
              </span>
            </div>

            <p>
              <strong>Both can be at the same address</strong>, and RGB will still know which UTXO 
              holds which assets. Address is just for receiving; UTXO identity is what matters for RGB.
            </p>
          </div>

          <div>
            <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
              UTXO Amount Management Strategy
            </h3>
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                <thead className="bg-gray-50 dark:bg-gray-700">
                  <tr>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                      UTXO Type
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                      Amount
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">
                      Purpose
                    </th>
                  </tr>
                </thead>
                <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                  <tr>
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white">
                      Small (30k)
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-700 dark:text-gray-300">
                      ~0.0003 BTC
                    </td>
                    <td className="px-6 py-4 text-sm text-gray-700 dark:text-gray-300">
                      <strong>RGB operations</strong> - bind assets, transfer tokens
                    </td>
                  </tr>
                  <tr>
                    <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white">
                      Large (1M+)
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-700 dark:text-gray-300">
                      Remaining
                    </td>
                    <td className="px-6 py-4 text-sm text-gray-700 dark:text-gray-300">
                      <strong>Fee reservoir</strong> - pay transaction fees, daily spending
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p className="mt-4 text-sm text-gray-600 dark:text-gray-400">
              This is like having <strong>$30 bills</strong> (small UTXOs) for specific RGB transactions 
              and <strong>$1000 bills</strong> (large UTXOs) for general use.
            </p>
          </div>

          <div>
            <h3 className="text-xl font-semibold text-gray-900 dark:text-white mb-2">
              Your Wallet After Creating RGB UTXO
            </h3>
            <div className="bg-gray-50 dark:bg-gray-700 rounded p-4 font-mono text-sm">
              Address #1:<br />
              ├─ UTXO 35d86d6a:0 = 30,000 sats<br />
              │  └─ Purpose: Bind RGB tokens here later<br />
              │  └─ Status: Unoccupied (ready for RGB)<br />
              │<br />
              └─ UTXO 35d86d6a:1 = 1,069,738 sats<br />
              &nbsp;&nbsp;&nbsp;└─ Purpose: Pay fees for RGB transactions<br />
              &nbsp;&nbsp;&nbsp;└─ Status: Unoccupied (free to spend)
            </div>
            <p className="mt-4 text-green-600 dark:text-green-400 font-semibold">
              ✅ This works perfectly for RGB!
            </p>
            <p className="mt-2">
              When you issue tokens, you'll bind them to <code className="bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded">35d86d6a:0</code>, 
              and use <code className="bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded">35d86d6a:1</code> to 
              pay the transaction fee.
            </p>
          </div>

          <div className="bg-yellow-50 dark:bg-yellow-900/20 border-l-4 border-yellow-500 p-4">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
              💡 TL;DR
            </h3>
            <ul className="space-y-2 text-sm">
              <li>✅ <strong>Why create separate UTXOs?</strong> Isolate RGB assets to small UTXOs, keep larger UTXOs free for spending/fees</li>
              <li>✅ <strong>Why same address is fine?</strong> RGB tracks by txid:vout, not address. Different vouts = different seals</li>
              <li>✅ <strong>What our wallet does:</strong> Creates 30k UTXO for RGB and change UTXO for fees, both ready for RGB operations</li>
            </ul>
          </div>
        </div>
      </section>

      {/* Additional Resources */}
      <section className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-6">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-4">
          📖 Additional Resources
        </h2>
        <ul className="space-y-2 text-gray-700 dark:text-gray-300">
          <li>
            <a
              href="https://rgb.tech"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary dark:text-blue-400 hover:underline"
            >
              RGB Protocol Official Documentation →
            </a>
          </li>
          <li>
            <a
              href="https://github.com/RGB-WG"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary dark:text-blue-400 hover:underline"
            >
              RGB Working Group GitHub →
            </a>
          </li>
          <li>
            <a
              href="https://mempool.space/signet"
              target="_blank"
              rel="noopener noreferrer"
              className="text-primary dark:text-blue-400 hover:underline"
            >
              Signet Mempool Explorer →
            </a>
          </li>
        </ul>
      </section>
    </div>
  );
}

