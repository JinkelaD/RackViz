import { useCallback } from 'react';
import { Rack } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

export function useRacks() {
  const { items: racks, loading, refresh, create, update, remove } = useApiList<Rack>(
    () => api.listRacks() as Promise<Rack[]>,
    (data) => api.createRack(data as api.RackCreate) as Promise<Rack>,
    (id, data) => api.updateRack(id, data as api.RackUpdate) as Promise<Rack | null>,
    (id) => api.deleteRack(id),
  );

  const updateQuiet = useCallback(async (id: number, data: Partial<Rack>) => {
    return api.updateRack(id, data as api.RackUpdate);
  }, []);

  return { racks, loading, refresh, create, update, updateQuiet, remove };
}
