import { useState, useCallback, useMemo, useEffect } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useRacks } from '../hooks/useRacks';
import { useRooms } from '../hooks/useRooms';
import { useDevices } from '../hooks/useDevices';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { useTheme } from '../contexts/ThemeContext';
import { ViewMode, Room } from '../types';
import * as tauriApi from '../tauri-api';

export interface LayoutContext {
  view: ViewMode;
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  showAddRack: boolean;
  setShowAddRack: (v: boolean) => void;
  selectedRoomId: number | null;
  rooms: Room[];
  setSelectedRoomId: (id: number | null) => void;
  showRoomInput: boolean;
  setShowRoomInput: (v: boolean) => void;
  editingRoomId: number | null;
  setEditingRoomId: (id: number | null) => void;
  roomName: string;
  setRoomName: (name: string) => void;
  handleAddRoom: () => void;
  handleEditRoom: () => void;
  handleDeleteRoom: (id: number) => void;
  startEditRoom: (id: number, name: string) => void;
  updateRoom: (id: number, data: Partial<Room>) => Promise<void>;
}

export default function Layout() {
  const navigate = useNavigate();
  const location = useLocation();
  const { racks, updateQuiet: updateRackQuiet } = useRacks();
  const { rooms, create: createRoom, update: updateRoom, remove: removeRoom } = useRooms();
  const { devices } = useDevices();
  const { models } = useDeviceModels();

  const [view, setView] = useState<ViewMode>('front');
  const [zoom, setZoom] = useState(100);
  const [searchQuery, setSearchQuery] = useState('');
  const [showAddRack, setShowAddRack] = useState(false);
  const [selectedRoomId, setSelectedRoomId] = useState<number | null>(null);
  const [showRoomInput, setShowRoomInput] = useState(false);
  const [editingRoomId, setEditingRoomId] = useState<number | null>(null);
  const [roomName, setRoomName] = useState('');

  // 设置弹窗 & 日志配置
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [loggingEnabled, setLoggingEnabled] = useState(false);
  const [logDir, setLogDir] = useState('');
  const [loggingLoading, setLoggingLoading] = useState(false);

  const onZoomIn = useCallback(() => setZoom(z => Math.min(z + 10, 200)), []);
  const onZoomOut = useCallback(() => setZoom(z => Math.max(z - 10, 50)), []);
  const onZoomReset = useCallback(() => setZoom(100), []);

  const { theme: currentTheme, toggle: toggleTheme } = useTheme();

  const handleAddRack = useCallback(() => setShowAddRack(true), []);

  // 打开设置时加载日志配置
  useEffect(() => {
    if (settingsOpen) {
      tauriApi.getLoggingConfig().then(cfg => {
        setLoggingEnabled(cfg.enabled);
        setLogDir(cfg.logDir);
      }).catch(console.error);
    }
  }, [settingsOpen]);

  const handleToggleLogging = async (checked: boolean) => {
    setLoggingLoading(true);
    try {
      const cfg = await tauriApi.setLoggingEnabled(checked);
      setLoggingEnabled(cfg.enabled);
      setLogDir(cfg.logDir);
    } catch (err) {
      console.error('切换日志失败:', err);
    } finally {
      setLoggingLoading(false);
    }
  };

  const handleOpenLogDir = async () => {
    try {
      await tauriApi.openLogDir();
    } catch (err) {
      console.error('打开日志目录失败:', err);
    }
  };

  const navItems = [
    { path: '/racks', label: '机柜管理' },
    { path: '/devices', label: '设备台账' },
  ];


  const handleSelectRoom = (id: number | null) => {
    setSelectedRoomId(prev => prev === id ? null : id);
  };

  const handleAddRoom = async () => {
    if (!roomName.trim()) return;
    await createRoom({ name: roomName.trim(), sort_order: rooms.length });
    setRoomName('');
    setShowRoomInput(false);
  };

  const handleEditRoom = async () => {
    if (!roomName.trim() || editingRoomId === null) return;
    await updateRoom(editingRoomId, { name: roomName.trim() });
    setRoomName('');
    setEditingRoomId(null);
    setShowRoomInput(false);
  };

  const handleDeleteRoom = async (id: number) => {
    if (selectedRoomId === id) setSelectedRoomId(null);
    await Promise.all(
      racks.filter(r => r.room_id === id).map(r =>
        updateRackQuiet(r.id, { room_id: null })
      )
    );
    await removeRoom(id);
  };

  const startEditRoom = (id: number, name: string) => {
    setEditingRoomId(id);
    setRoomName(name);
    setShowRoomInput(true);
  };

  const ctxValue: LayoutContext = useMemo(() => ({
    view, zoom, onZoomIn, onZoomOut, onZoomReset, searchQuery, onSearchChange: setSearchQuery, showAddRack, setShowAddRack, selectedRoomId,
    rooms,
    setSelectedRoomId,
    showRoomInput,
    setShowRoomInput,
    editingRoomId,
    setEditingRoomId,
    roomName,
    setRoomName,
    handleAddRoom,
    handleEditRoom,
    handleDeleteRoom,
    startEditRoom,
    updateRoom,
  }), [view, zoom, onZoomIn, onZoomOut, onZoomReset, searchQuery, showAddRack, selectedRoomId, rooms, showRoomInput, editingRoomId, roomName]);

  return (
    <div id="app">
      <header id="header">
        <div className="logo">Rack<span>Viz</span></div>
        <span className="version">v1.2.0</span>
        <button className="header-btn" onClick={toggleTheme} title={currentTheme === 'dark' ? '切换亮色主题' : '切换暗色主题'}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            {currentTheme === 'dark' ? (
              <><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></>
            ) : (
              <><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></>
            )}
          </svg>
        </button>
        <button className="header-btn" onClick={() => setSettingsOpen(true)} title="设置">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
        </button>
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
              {item.label}
            </button>
          ))}
        </nav>
      </header>

      <main id="main">
        <Outlet context={ctxValue} />
      </main>

      {/* 设置弹窗 */}
      {settingsOpen && (
        <div className="settings-overlay" onClick={() => setSettingsOpen(false)}>
          <div className="settings-modal" onClick={e => e.stopPropagation()}>
            <div className="settings-header">
              <h3>设置</h3>
              <button className="settings-close" onClick={() => setSettingsOpen(false)}>✕</button>
            </div>
            <div className="settings-body">
              <div className="settings-section">
                <div className="settings-section-title">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>
                  </svg>
                  日志收集
                </div>
                <p className="settings-desc">
                  开启后，系统运行日志将写入本地文件，便于问题排查和 Bug 收集。
                  日志文件按天轮转，最多保留 7 天。
                </p>
                <div className="settings-row">
                  <label className="settings-switch">
                    <input
                      type="checkbox"
                      checked={loggingEnabled}
                      onChange={e => handleToggleLogging(e.target.checked)}
                      disabled={loggingLoading}
                    />
                    <span className="settings-slider" />
                  </label>
                  <span className="settings-label">{loggingEnabled ? '已开启' : '已关闭'}</span>
                </div>
                {logDir && (
                  <div className="settings-logdir">
                    <span className="settings-logdir-path" title={logDir}>{logDir}</span>
                    <button className="settings-logdir-btn" onClick={handleOpenLogDir} title="打开日志目录">
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>
                      </svg>
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
