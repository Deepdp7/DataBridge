import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout } from './pages/Layout';
import { Dashboard } from './pages/Dashboard';
import { Symbols } from './pages/Symbols';
import { LiveMonitor } from './pages/LiveMonitor';
import { History } from './pages/History';
import { Logs } from './pages/Logs';
import { Settings } from './pages/Settings';

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Dashboard />} />
          <Route path="symbols" element={<Symbols />} />
          <Route path="live" element={<LiveMonitor />} />
          <Route path="history" element={<History />} />
          <Route path="logs" element={<Logs />} />
          <Route path="settings" element={<Settings />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
