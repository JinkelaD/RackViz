import { useState, useRef } from 'react';
import { Room } from '../types';

interface RoomTabsProps {
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

export default function RoomTabs({
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
}: RoomTabsProps) {
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
