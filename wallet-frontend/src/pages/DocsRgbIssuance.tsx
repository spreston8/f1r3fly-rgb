import DocsLayout from '../components/DocsLayout';

export default function DocsRgbIssuance() {
  return (
    <DocsLayout>
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow dark:shadow-gray-900 p-8">
        <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-6">
          🎬 What Happens After RGB Asset Issuance?
        </h1>

        {/* Immediate Contract Creation */}
        <section className="mb-8">
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
            1️⃣ Immediate: Contract Created Locally
          </h2>
          <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 mb-4">
            <p className="text-gray-700 dark:text-gray-300 mb-2">
              When you click "Issue Asset", the RGB contract is created <strong>instantly</strong>:
            </p>
            <ul className="list-disc list-inside space-y-1 text-gray-600 dark:text-gray-400 ml-4">
              <li>✅ RGB contract is created client-side</li>
              <li>✅ Stored in: <code className="text-sm bg-gray-200 dark:bg-gray-600 px-1 rounded">./wallets/rgb_data/</code></li>
              <li>✅ Contract state bound to the UTXO seal</li>
              <li>✅ Returns contract ID instantly</li>
            </ul>
          </div>
          <div className="bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-lg p-4">
            <p className="text-blue-800 dark:text-blue-300">
              <strong>Key Point:</strong> No Bitcoin transaction is created! The asset is created entirely client-side.
            </p>
          </div>
        </section>

        {/* Where Asset Lives */}
        <section className="mb-8">
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
            2️⃣ Where the Asset Lives
          </h2>
          <div className="bg-gray-900 dark:bg-gray-950 text-green-400 rounded-lg p-4 mb-4 font-mono text-sm overflow-x-auto">
            <pre>{`./wallets/
├── rgb_data/              ← RGB contracts live here
    ├── RGB20-FNA.issuer   (schema)
    └── bitcoin.testnet/
        └── contracts/
            └── rgb:abc123...  (YOUR ASSET!)
                ├── Ticker: MTK
                ├── Name: MyToken
                ├── Supply: 1,000,000
                └── Seal: txid:vout`}</pre>
          </div>
          <p className="text-gray-700 dark:text-gray-300">
            The asset exists <strong>only on your machine</strong> at this point. RGB is "client-side validated."
          </p>
        </section>

        {/* Is There a Pending State? */}
        <section className="mb-8">
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
            3️⃣ Is There a "Pending" State? NO! ❌
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
            <div className="bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-700 rounded-lg p-4">
              <h3 className="font-semibold text-orange-900 dark:text-orange-300 mb-2">Bitlight's Approach:</h3>
              <ol className="list-decimal list-inside space-y-1 text-sm text-orange-800 dark:text-orange-400">
                <li>User clicks "Issue Asset"</li>
                <li>Creates NEW Bitcoin transaction</li>
                <li>Broadcasts to network</li>
                <li>Wait for confirmation (10+ mins)</li>
                <li>Asset becomes active</li>
              </ol>
            </div>
            <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-700 rounded-lg p-4">
              <h3 className="font-semibold text-green-900 dark:text-green-300 mb-2">Our Approach:</h3>
              <ol className="list-decimal list-inside space-y-1 text-sm text-green-800 dark:text-green-400">
                <li>User clicks "Issue Asset"</li>
                <li>Uses EXISTING UTXO (already on-chain)</li>
                <li>Creates contract locally ← instant</li>
                <li>✅ Asset is ACTIVE immediately</li>
                <li>No waiting needed</li>
              </ol>
            </div>
          </div>
          <div className="bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-700 rounded-lg p-4">
            <p className="text-green-800 dark:text-green-300">
              Since we're binding to an <strong>existing UTXO</strong> that's already confirmed on-chain, there's no pending state!
            </p>
          </div>
        </section>

        {/* Where You'll See It */}
        <section className="mb-8">
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
            4️⃣ Where You'll See It in the UI
          </h2>
          <p className="text-gray-700 dark:text-gray-300 mb-4">
            After issuing, when you close the success modal (which triggers wallet refresh):
          </p>
          
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Step A: Success Modal Shows</h3>
              <div className="bg-gray-100 dark:bg-gray-700 rounded-lg p-4">
                <p className="text-sm text-gray-600 dark:text-gray-400 mb-2">🎉 Asset Issued Successfully!</p>
                <p className="text-xs text-gray-500 dark:text-gray-500 font-mono">Contract ID: rgb:abc123def... [Copy]</p>
                <p className="text-xs text-gray-500 dark:text-gray-500 font-mono">Genesis Seal: abc123...xyz:0</p>
              </div>
            </div>

            <div>
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Step B: UTXO Tab Updates (Immediate)</h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <p className="text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">BEFORE ISSUANCE:</p>
                  <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-700 rounded-lg p-3 text-sm">
                    <p className="font-semibold text-green-900 dark:text-green-300">Unoccupied (1)</p>
                    <p className="text-xs text-green-700 dark:text-green-400">✓ Available</p>
                    <p className="text-xs text-gray-600 dark:text-gray-400">abc123...xyz:0</p>
                    <p className="text-xs text-gray-600 dark:text-gray-400">30,000 sats</p>
                  </div>
                </div>
                <div>
                  <p className="text-sm font-medium text-gray-600 dark:text-gray-400 mb-2">AFTER ISSUANCE:</p>
                  <div className="bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-700 rounded-lg p-3 text-sm">
                    <p className="font-semibold text-orange-900 dark:text-orange-300">Occupied (1)</p>
                    <p className="text-xs text-orange-700 dark:text-orange-400">🔒 RGB Asset</p>
                    <p className="text-xs text-gray-600 dark:text-gray-400">abc123...xyz:0</p>
                    <p className="text-xs text-gray-600 dark:text-gray-400">30,000 sats</p>
                    <p className="text-xs font-semibold text-gray-700 dark:text-gray-300 mt-2">Bound RGB Assets:</p>
                    <p className="text-xs text-gray-600 dark:text-gray-400">MTK • MyToken • 1,000,000</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Layer 1 Confirmation */}
        <section className="mb-8">
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
            5️⃣ What About Layer 1 Confirmation?
          </h2>
          <p className="text-gray-700 dark:text-gray-300 mb-4">
            <strong>The UTXO is already confirmed!</strong>
          </p>
          <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4 mb-4">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Timeline:</h3>
            <div className="space-y-2 text-sm text-gray-600 dark:text-gray-400">
              <div>
                <p className="font-medium text-gray-700 dark:text-gray-300">Earlier:</p>
                <ul className="list-disc list-inside ml-4">
                  <li>You created UTXO via "Create UTXO"</li>
                  <li>Bitcoin TX broadcast</li>
                  <li>TX confirmed (e.g., 5 confirmations)</li>
                  <li>✓ UTXO exists on-chain</li>
                </ul>
              </div>
              <div>
                <p className="font-medium text-gray-700 dark:text-gray-300">Now (Asset Issuance):</p>
                <ul className="list-disc list-inside ml-4">
                  <li>RGB contract created locally</li>
                  <li>Contract bound to existing UTXO</li>
                  <li>NO new Bitcoin TX needed</li>
                  <li>Asset is immediately usable ✓</li>
                </ul>
              </div>
            </div>
          </div>
          <div className="bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700 rounded-lg p-4">
            <p className="text-blue-800 dark:text-blue-300">
              <strong>The asset is immediately usable</strong> because the UTXO it's bound to is already confirmed on Layer 1, and RGB state is local (not on blockchain).
            </p>
          </div>
        </section>

        {/* When TX Needed */}
        <section className="mb-8">
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
            6️⃣ When WOULD You Need a Bitcoin Transaction?
          </h2>
          <p className="text-gray-700 dark:text-gray-300 mb-4">
            You'll need a Bitcoin TX when you want to <strong>transfer</strong> the tokens:
          </p>
          <div className="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-2">RGB Transfer Flow:</h3>
            <ol className="list-decimal list-inside space-y-1 text-sm text-gray-600 dark:text-gray-400">
              <li>Spend the "occupied" UTXO in a Bitcoin TX</li>
              <li>Create new outputs (UTXOs) for:
                <ul className="list-disc list-inside ml-6">
                  <li>Recipient's tokens (new seal)</li>
                  <li>Your change tokens (new seal)</li>
                </ul>
              </li>
              <li>Create RGB consignment (proof of transfer)</li>
              <li>Send consignment + Bitcoin TX ID to recipient</li>
              <li>Recipient validates and accepts</li>
            </ol>
          </div>
        </section>

        {/* Summary */}
        <section className="mb-8">
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
            🎯 Summary
          </h2>
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Where Will You See It?</h3>
              <ul className="list-disc list-inside space-y-1 text-gray-700 dark:text-gray-300 ml-4">
                <li>✅ <strong>Success modal</strong> - Shows contract ID immediately</li>
                <li>✅ <strong>UTXO tab</strong> - UTXO moves from "Unoccupied" to "Occupied"</li>
                <li>✅ <strong>Bound Assets section</strong> - Shows ticker, name, amount, contract ID</li>
              </ul>
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 dark:text-white mb-2">Is It Pending?</h3>
              <p className="text-gray-700 dark:text-gray-300">
                ❌ <strong>NO!</strong> The asset is <strong>active immediately</strong> because:
              </p>
              <ul className="list-disc list-inside space-y-1 text-gray-600 dark:text-gray-400 ml-4">
                <li>UTXO already exists on-chain (confirmed)</li>
                <li>RGB contract is created locally (instant)</li>
                <li>No Bitcoin transaction broadcast needed</li>
              </ul>
            </div>
          </div>
        </section>

        {/* Comparison Table */}
        <section>
          <h2 className="text-2xl font-semibold text-gray-900 dark:text-white mb-4">
            📊 Our Approach vs. Bitlight
          </h2>
          <div className="overflow-x-auto">
            <table className="min-w-full border border-gray-300 dark:border-gray-600">
              <thead className="bg-gray-100 dark:bg-gray-700">
                <tr>
                  <th className="px-4 py-2 text-left text-sm font-semibold text-gray-900 dark:text-white border-b border-gray-300 dark:border-gray-600">Aspect</th>
                  <th className="px-4 py-2 text-left text-sm font-semibold text-gray-900 dark:text-white border-b border-gray-300 dark:border-gray-600">Our Implementation</th>
                  <th className="px-4 py-2 text-left text-sm font-semibold text-gray-900 dark:text-white border-b border-gray-300 dark:border-gray-600">Bitlight</th>
                </tr>
              </thead>
              <tbody className="text-sm">
                <tr className="border-b border-gray-200 dark:border-gray-700">
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Speed</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Instant ⚡</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Wait for confirmation ⏳</td>
                </tr>
                <tr className="border-b border-gray-200 dark:border-gray-700">
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Cost</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Free 🆓</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Requires Bitcoin TX fee 💰</td>
                </tr>
                <tr className="border-b border-gray-200 dark:border-gray-700">
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">UTXO</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Use existing UTXO ♻️</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Creates new UTXO 🆕</td>
                </tr>
                <tr className="border-b border-gray-200 dark:border-gray-700">
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">On-chain footprint</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">None</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">New Bitcoin TX 📝</td>
                </tr>
                <tr>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Transferability</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Fully transferable ✓</td>
                  <td className="px-4 py-2 text-gray-700 dark:text-gray-300">Fully transferable ✓</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-4">
            Both approaches create valid RGB assets that can be transferred later. The difference is just the genesis method! 🎨
          </p>
        </section>
      </div>
    </DocsLayout>
  );
}
