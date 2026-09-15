import { Routes, Route, Navigate } from 'react-router-dom';
import Layout from './components/Layout';
import RackView from './pages/RackView';
import DeviceList from './pages/DeviceList';
import { UndoProvider } from './contexts/UndoContext';
import { GlobalShortcutsProvider } from './hooks/useGlobalShortcuts';

export default function App() {
  return (
    <UndoProvider>
      <GlobalShortcutsProvider>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/racks" element={<RackView />} />
            <Route path="/devices" element={<DeviceList />} />
            <Route path="*" element={<Navigate to="/racks" replace />} />
          </Route>
        </Routes>
      </GlobalShortcutsProvider>
    </UndoProvider>
  );
}
