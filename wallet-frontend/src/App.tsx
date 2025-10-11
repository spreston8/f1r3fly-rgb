import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import Layout from './components/Layout';
import Home from './pages/Home';
import CreateWallet from './pages/CreateWallet';
import ImportWallet from './pages/ImportWallet';
import WalletDetail from './pages/WalletDetail';
import DocsUtxoCreation from './pages/DocsUtxoCreation';
import DocsRgbIssuance from './pages/DocsRgbIssuance';
import DocsResources from './pages/DocsResources';

function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/create" element={<CreateWallet />} />
          <Route path="/import" element={<ImportWallet />} />
          <Route path="/wallet/:name" element={<WalletDetail />} />
          <Route path="/docs" element={<Navigate to="/docs/utxo-creation" replace />} />
          <Route path="/docs/utxo-creation" element={<DocsUtxoCreation />} />
          <Route path="/docs/rgb-issuance" element={<DocsRgbIssuance />} />
          <Route path="/docs/resources" element={<DocsResources />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}

export default App;
