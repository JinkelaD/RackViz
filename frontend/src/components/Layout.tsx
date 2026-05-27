import { useState, useCallback } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useRacks } from '../hooks/useRacks';
import { useDevices } from '../hooks/useDevices';
import { EditMode, ViewMode } from '../types';
import Toolbar from './Toolbar';
import StatusBar from './StatusBar';

export default function Layout() {
  const navigate = useNavigate();
  const location = useLocation();
  const { racks } = useRacks();
  const { devices } = useDevices();
  
  const [mode, setMode] = useState<EditMode>('drag');
  const [view, setView] = useState<ViewMode>('front');
  const [zoom, setZoom] = useState(100);
  const [searchQuery, setSearchQuery] = useState('');

  const onZoomIn = useCallback(() => setZoom(z => Math.min(z + 10, 200)), []);
  const onZoomOut = useCallback(() => setZoom(z => Math.max(z - 10, 50)), []);
  const onZoomReset = useCallback(() => setZoom(100), []);

  const onlineCount = devices.filter(d => d.status === 'online').length;
  const totalPower = devices.reduce((sum, d) => sum + (d.power_watt || 0), 0);

  const navItems = [
    { path: '/racks', label: '机柜管理', icon: 'rack' },
    { path: '/devices', label: '设备台账', icon: 'devices' },
    { path: '/models', label: '设备库', icon: 'models' },
  ];

  return (
    <div id="app">
      <header id="header">
        <div className="logo">Rack<span>Viz</span></div>
        <span className="version">v1.0</span>
        <nav className="nav-tabs">
          {navItems.map(item => (
            <button
              key={item.path}
              className={`nav-tab ${location.pathname === item.path ? 'active' : ''}`}
              onClick={() => navigate(item.path)}
            >
              {item.path === '/racks' && (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="8" x2="22" y2="8"/><line x1="2" y1="16" x2="22" y2="16"/><line x1="8" y1="2" x2="8" y2="22"/>
                </svg>
              )}
              {item.path === '/devices' && (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="6" x2="22" y2="6"/><line x1="2" y1="10" x2="22" y2="10"/><line x1="2" y1="14" x2="22" y2="14"/><line x1="2" y1="18" x2="22" y2="18"/>
                </svg>
              )}
              {item.path === '/models' && (
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="2" y="2" width="20" height="20" rx="2"/><path d="M12 6v12"/><path d="M6 12h12"/>
                </svg>
              )}
              {item.label}
              {item.path === '/racks' && (
                <span className="badge">{racks.length}</span>
              )}
            </button>
          ))}
        </nav>
        <div className="header-actions">
          <button className="header-btn accent" title="IP探测">
            <span className="dot on"></span> ICMP
          </button>
          <button className="header-btn" title="功率显示">
            <span className="dot off"></span> 功率
          </button>
          <button className="header-btn">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/>
            </svg> 导出
          </button>
          <button className="header-btn">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <rect x="2" y="2" width="20" height="20" rx="2"/><path d="M9 3v4"/><path d="M15 3v4"/>
            </svg>
          </button>
        </div>
      </header>

      {location.pathname === '/racks' && (
        <Toolbar
          mode={mode}
          onModeChange={setMode}
          view={view}
          onViewChange={setView}
          zoom={zoom}
          onZoomIn={onZoomIn}
          onZoomOut={onZoomOut}
          onZoomReset={onZoomReset}
          searchQuery={searchQuery}
          onSearchChange={setSearchQuery}
        />
      )}

      <main id="main">
        <Outlet context={{ mode, view, zoom, searchQuery }} />
      </main>

      {location.pathname === '/racks' && (
        <StatusBar
          zoom={zoom}
          rackCount={racks.length}
          deviceCount={devices.length}
          onlineCount={onlineCount}
          totalPower={totalPower}
          mode={mode}
        />
      )}
    </div>
  );
}