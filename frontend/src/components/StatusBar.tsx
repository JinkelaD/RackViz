import { useState, useRef } from 'react';
import { Room } from '../types';

interface Props {
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  onAddRack: () => void;
  deviceCount: number;
  onlineCount: number;
  totalUUsed: number;
  totalU: number;
  rooms: Room[];
  selectedRoomId: number | null;
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

export function RoomTabs({
  rooms,
  selectedRoomId,
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
}: Omit<Props, 'zoom' | 'onZoomIn' | 'onZoomOut' | 'onZoomReset' | 'searchQuery' | 'onSearchChange' | 'onAddRack' | 'rackCount' | 'deviceCount' | 'onlineCount' | 'totalUUsed' | 'totalU' | 'contextLabel'>) {
  const [draggedRoomId, setDraggedRoomId] = useState<number | null>(null);
  const [dragOverRoomId, setDragOverRoomId] = useState<number | null>(null);
  const roomsRef = useRef(rooms);
  roomsRef.current = rooms;

  const handleSelectRoom = (id: number | null) => {
    setSelectedRoomId(id);
  };

  const handleDragStart = (e: React.DragEvent, roomId: number) => {
    setDraggedRoomId(roomId);
    e.dataTransfer.effectAllowed = 'move';
  };

  const handleDragOver = (e: React.DragEvent, roomId: number) => {
    e.preventDefault();
    if (draggedRoomId !== roomId) {
      setDragOverRoomId(roomId);
    }
  };

  const handleDragLeave = () => {
    setDragOverRoomId(null);
  };

  const handleDrop = async (e: React.DragEvent, targetRoomId: number) => {
    e.preventDefault();
    if (!draggedRoomId || draggedRoomId === targetRoomId) {
      setDraggedRoomId(null);
      setDragOverRoomId(null);
      return;
    }

    const currentRooms = [...roomsRef.current].sort((a, b) => a.sort_order - b.sort_order);
    const draggedIndex = currentRooms.findIndex(r => r.id === draggedRoomId);
    const targetIndex = currentRooms.findIndex(r => r.id === targetRoomId);

    if (draggedIndex === -1 || targetIndex === -1) {
      setDraggedRoomId(null);
      setDragOverRoomId(null);
      return;
    }

    const newOrders = currentRooms.map((room, index) => {
      if (room.id === draggedRoomId) {
        return { ...room, sort_order: targetIndex };
      } else if (draggedIndex < targetIndex) {
        if (index > draggedIndex && index <= targetIndex) {
          return { ...room, sort_order: index - 1 };
        }
      } else {
        if (index >= targetIndex && index < draggedIndex) {
          return { ...room, sort_order: index + 1 };
        }
      }
      return room;
    });

    await Promise.all(
      newOrders.map(room => updateRoom(room.id, { sort_order: room.sort_order }))
    );

    setDraggedRoomId(null);
    setDragOverRoomId(null);
  };

  const sortedRooms = [...rooms].sort((a, b) => a.sort_order - b.sort_order);

  return (
    <div className="room-tabs">
      <button
        className={`room-tab ${selectedRoomId === null ? 'active' : ''}`}
        onClick={() => handleSelectRoom(null)}
      >
        全部机房
      </button>
      {sortedRooms.map(room => (
        <div
          key={room.id}
          className={`room-tab-wrapper ${draggedRoomId === room.id ? 'dragging' : ''} ${dragOverRoomId === room.id ? 'drag-over' : ''}`}
          title={room.location || room.name}
          draggable
          onDragStart={(e) => handleDragStart(e, room.id)}
          onDragOver={(e) => handleDragOver(e, room.id)}
          onDragLeave={handleDragLeave}
          onDrop={(e) => handleDrop(e, room.id)}
        >
          <button
            className={`room-tab ${selectedRoomId === room.id ? 'active' : ''}`}
            onClick={() => handleSelectRoom(room.id)}
            onDoubleClick={() => startEditRoom(room.id, room.name)}
          >
            {room.name}
          </button>
          <button
            className="room-tab-close"
            onClick={(e) => { e.stopPropagation(); handleDeleteRoom(room.id); }}
            title="删除机房"
          >
            ×
          </button>
        </div>
      ))}
      {showRoomInput ? (
        <div className="room-tab-input-wrapper">
          <input
            className="room-tab-input"
            value={roomName}
            onChange={e => setRoomName(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') editingRoomId ? handleEditRoom() : handleAddRoom();
              if (e.key === 'Escape') { setShowRoomInput(false); setEditingRoomId(null); setRoomName(''); }
            }}
            placeholder="机房名称"
            autoFocus
          />
          <button className="room-tab-btn" onClick={editingRoomId ? handleEditRoom : handleAddRoom}>✓</button>
          <button className="room-tab-btn" onClick={() => { setShowRoomInput(false); setEditingRoomId(null); setRoomName(''); }}>×</button>
        </div>
      ) : (
        <button className="room-tab-add" onClick={() => { setEditingRoomId(null); setRoomName(''); setShowRoomInput(true); }} title="新建机房">
          + 新建机房
        </button>
      )}
    </div>
  );
}

export default function StatusBar({
  zoom, onZoomIn, onZoomOut, onZoomReset,
  searchQuery, onSearchChange,
  onAddRack,
  deviceCount, onlineCount, totalUUsed, totalU,
}: Omit<Props, 'rooms' | 'selectedRoomId' | 'setSelectedRoomId' | 'showRoomInput' | 'setShowRoomInput' | 'editingRoomId' | 'setEditingRoomId' | 'roomName' | 'setRoomName' | 'handleAddRoom' | 'handleEditRoom' | 'handleDeleteRoom' | 'startEditRoom' | 'updateRoom'>) {
  const uPct = totalU > 0 ? Math.round((totalUUsed / totalU) * 100) : 0;

  return (
    <div className="statusbar">
      <div className="statusbar-left">
        <button className="statusbar-btn accent" onClick={onAddRack} title="添加机柜">
          + 机柜
        </button>

        <div className="statusbar-segment">
          <button className="statusbar-btn" onClick={onZoomOut} title="缩小">−</button>
          <button className="statusbar-btn zoom" onClick={onZoomReset}>{zoom}%</button>
          <button className="statusbar-btn" onClick={onZoomIn} title="放大">+</button>
        </div>

        <div className="statusbar-segment statusbar-search">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <input
            type="text"
            placeholder="搜索..."
            value={searchQuery}
            onChange={e => onSearchChange(e.target.value)}
          />
          {searchQuery && (
            <button className="statusbar-search-clear" onClick={() => onSearchChange('')}>
              ×
            </button>
          )}
        </div>
      </div>

      <div className="statusbar-right">
        <span className="statusbar-item">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="6" x2="22" y2="6"/><line x1="2" y1="10" x2="22" y2="10"/><line x1="2" y1="14" x2="22" y2="14"/></svg>
          {deviceCount} 设备
          <span className="statusbar-highlight">· {onlineCount} 在线</span>
        </span>
        <span className="statusbar-sep" />
        <span className="statusbar-item">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="8" x2="22" y2="8"/></svg>
          {totalUUsed}/{totalU}U
          <span className={`statusbar-pct ${uPct > 85 ? 'warn' : ''}`}>{uPct}%</span>
        </span>
      </div>
    </div>
  );
}
