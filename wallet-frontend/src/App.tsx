import { BrowserRouter, Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Home from './pages/Home';
import CreateWallet from './pages/CreateWallet';
import ImportWallet from './pages/ImportWallet';
import WalletDetail from './pages/WalletDetail';

function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/create" element={<CreateWallet />} />
          <Route path="/import" element={<ImportWallet />} />
          <Route path="/wallet/:name" element={<WalletDetail />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}

export default App;
