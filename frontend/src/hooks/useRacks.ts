import { useState, useEffect, useCallback } from 'react';
import { Rack } from '../types';
import { rackApi } from '../api/client';

export function useRacks() {
  const [racks, setRacks] = useState<Rack[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(() => {
    setLoading(true);
    rackApi.list().then(setRacks).finally(() => setLoading(false));
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const create = async (data: Partial<Rack>) => {
    await rackApi.create(data);
    refresh();
  };

  const update = async (id: number, data: Partial<Rack>) => {
    await rackApi.update(id, data);
    refresh();
  };

  const remove = async (id: number) => {
    await rackApi.delete(id);
    refresh();
  };

  return { racks, loading, refresh, create, update, remove };
}