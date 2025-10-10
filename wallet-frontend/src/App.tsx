import { BrowserRouter, Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Home from './pages/Home';
import CreateWallet from './pages/CreateWallet';
import ImportWallet from './pages/ImportWallet';
import WalletDetail from './pages/WalletDetail';
import Docs from './pages/Docs';

function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/create" element={<CreateWallet />} />
          <Route path="/import" element={<ImportWallet />} />
          <Route path="/wallet/:name" element={<WalletDetail />} />
          <Route path="/docs" element={<Docs />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}

export default App;
