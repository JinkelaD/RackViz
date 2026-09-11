import { createContext, useContext, useMemo, useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import { useRooms } from '../hooks/useRooms';
import { useRacks } from '../hooks/useRacks';
import type { Room } from '../types';

/** 机房相关共享状态（Layout 级 Provider，字段 <10） */
export interface RoomContextValue {
  rooms: Room[];
  selectedRoomId: number | null;
  setSelectedRoomId: (id: number | null) => void;
  createRoom: (data: Partial<Room>) => Promise<void>;
  updateRoom: (id: number, data: Partial<Room>) => Promise<void>;
  /** 删除机房：先解除该机房下机柜的归属，再删除机房 */
  deleteRoom: (id: number) => Promise<void>;
}

export const RoomContext = createContext<RoomContextValue | undefined>(undefined);

export function useRoomContext(): RoomContextValue {
  const value = useContext(RoomContext);
  if (!value) {
    throw new Error('useRoomContext 必须在 <RoomProvider> 内使用');
  }
  return value;
}

/** 统一提供机房数据与 CRUD（单一数据源，避免各组件重复请求） */
export function RoomProvider({ children }: { children: ReactNode }) {
  const { rooms, create: createRoom, update: updateRoom, remove: removeRoom } = useRooms();
  const { racks, updateQuiet: updateRackQuiet } = useRacks();
  const [selectedRoomId, setSelectedRoomId] = useState<number | null>(null);

  const deleteRoom = useCallback(async (id: number) => {
    setSelectedRoomId(prev => (prev === id ? null : prev));
    // 该机房下机柜解除归属（删除前清空 racks.room_id 外键）
    await Promise.all(
      racks.filter(r => r.room_id === id).map(r => updateRackQuiet(r.id, { room_id: null })),
    );
    await removeRoom(id);
  }, [racks, updateRackQuiet, removeRoom]);

  const value = useMemo<RoomContextValue>(() => ({
    rooms,
    selectedRoomId,
    setSelectedRoomId,
    createRoom,
    updateRoom,
    deleteRoom,
  }), [rooms, selectedRoomId, createRoom, updateRoom, deleteRoom]);

  return <RoomContext.Provider value={value}>{children}</RoomContext.Provider>;
}
