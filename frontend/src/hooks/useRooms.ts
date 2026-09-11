import { Room } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

export function useRooms() {
  const { items: rooms, loading, refresh, create, update, remove } = useApiList<Room>(
    () => api.listRooms(),
    (data) => api.createRoom(data as api.RoomCreate),
    (id, data) => api.updateRoom(id, data),
    (id) => api.deleteRoom(id),
  );
  return { rooms, loading, refresh, create, update, remove };
}
