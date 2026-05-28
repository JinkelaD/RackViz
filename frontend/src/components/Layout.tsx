import { useState, useCallback, useMemo } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useRacks } from '../hooks/useRacks';
import { useRooms } from '../hooks/useRooms';
import { ViewMode, Room } from '../types';

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

  const [view, setView] = useState<ViewMode>('front');
  const [zoom, setZoom] = useState(100);
  const [searchQuery, setSearchQuery] = useState('');
  const [showAddRack, setShowAddRack] = useState(false);
  const [selectedRoomId, setSelectedRoomId] = useState<number | null>(null);
  const [showRoomInput, setShowRoomInput] = useState(false);
  const [editingRoomId, setEditingRoomId] = useState<number | null>(null);
  const [roomName, setRoomName] = useState('');

  const onZoomIn = useCallback(() => setZoom(z => Math.min(z + 10, 200)), []);
  const onZoomOut = useCallback(() => setZoom(z => Math.max(z - 10, 50)), []);
  const onZoomReset = useCallback(() => setZoom(100), []);

  const handleAddRack = useCallback(() => setShowAddRack(true), []);

  const navItems = [
    { path: '/racks', label: '机柜管理', icon: 'rack' },
    { path: '/devices', label: '设备台账', icon: 'devices' },
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
              {item.label}
            </button>
          ))}
        </nav>
        <div className="header-actions">
          <button className="header-btn accent" title="IP探测">
            <span className="dot on"></span> ICMP
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

      <main id="main">
        <Outlet context={ctxValue} />
      </main>
    </div>
  );
}
