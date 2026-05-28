import { useState, useEffect, useCallback } from 'react';
import { Room } from '../types';
import { roomApi } from '../api/client';

export function useRooms() {
  const [rooms, setRooms] = useState<Room[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(() => {
    setLoading(true);
    roomApi.list().then(setRooms).finally(() => setLoading(false));
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const create = async (data: Partial<Room>) => {
    await roomApi.create(data);
    refresh();
  };

  const update = async (id: number, data: Partial<Room>) => {
    await roomApi.update(id, data);
    refresh();
  };

  const remove = async (id: number) => {
    await roomApi.delete(id);
    refresh();
  };

  return { rooms, loading, refresh, create, update, remove };
}
