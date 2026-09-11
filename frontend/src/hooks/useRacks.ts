import { useCallback } from 'react';
import { Rack } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

export function useRacks() {
  const { items: racks, loading, refresh, create, update, remove } = useApiList<Rack>(
    () => api.listRacks(),
    (data) => api.createRack(data as api.RackCreate),
    (id, data) => api.updateRack(id, data),
    (id) => api.deleteRack(id),
  );

  const updateQuiet = useCallback((id: number, data: Partial<Rack>) => {
    return api.updateRack(id, data);
  }, []);

  return { racks, loading, refresh, create, update, updateQuiet, remove };
}
