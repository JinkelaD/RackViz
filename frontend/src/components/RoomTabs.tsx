import { useState, useRef } from 'react';
import { useRoomContext } from '../contexts/RoomContext';

/**
 * 机房标签页：CRUD / 选中 / 双击改名 / 拖拽排序。
 * 数据来自 RoomProvider 单一数据源；输入框状态为本组件局部状态。
 */
export default function RoomTabs() {
  const { rooms, selectedRoomId, setSelectedRoomId, createRoom, updateRoom, deleteRoom } = useRoomContext();
  const [showRoomInput, setShowRoomInput] = useState(false);
  const [editingRoomId, setEditingRoomId] = useState<number | null>(null);
  const [roomName, setRoomName] = useState('');
  const [draggedRoomId, setDraggedRoomId] = useState<number | null>(null);
  const [dragOverRoomId, setDragOverRoomId] = useState<number | null>(null);
  const roomsRef = useRef(rooms);
  roomsRef.current = rooms;

  const handleSelectRoom = (id: number | null) => {
    setSelectedRoomId(id);
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
            onDoubleClick={() => startEdit(room.id, room.name)}
          >
            {room.name}
          </button>
          <button
            className="room-tab-close"
            onClick={(e) => { e.stopPropagation(); deleteRoom(room.id); }}
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
              if (e.key === 'Escape') cancelInput();
            }}
            placeholder="机房名称"
            autoFocus
          />
          <button className="room-tab-btn" onClick={editingRoomId ? handleEditRoom : handleAddRoom}>✓</button>
          <button className="room-tab-btn" onClick={cancelInput}>×</button>
        </div>
      ) : (
        <button className="room-tab-add" onClick={() => { setEditingRoomId(null); setRoomName(''); setShowRoomInput(true); }} title="新建机房">
          + 新建机房
        </button>
      )}
    </div>
  );

  function startEdit(id: number, name: string) {
    setEditingRoomId(id);
    setRoomName(name);
    setShowRoomInput(true);
  }

  function cancelInput() {
    setShowRoomInput(false);
    setEditingRoomId(null);
    setRoomName('');
  }
}
