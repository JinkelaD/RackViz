import { Room } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

export function useRooms() {
  const { items: rooms, loading, refresh, create, update, remove } = useApiList<Room>(
    () => api.listRooms() as Promise<Room[]>,
    (data) => api.createRoom(data as api.RoomCreate) as Promise<Room>,
    (id, data) => api.updateRoom(id, data as api.RoomUpdate) as Promise<Room | null>,
    (id) => api.deleteRoom(id),
  );
  return { rooms, loading, refresh, create, update, remove };
}
