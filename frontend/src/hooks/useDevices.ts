import { useState, useEffect, useCallback } from 'react';
import { Device } from '../types';
import { deviceApi } from '../api/client';

export function useDevices() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(() => {
    setLoading(true);
    deviceApi.list().then(setDevices).finally(() => setLoading(false));
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const create = async (data: Partial<Device>) => {
    await deviceApi.create(data);
    refresh();
  };

  const update = async (id: number, data: Partial<Device>) => {
    await deviceApi.update(id, data);
    refresh();
  };

  const remove = async (id: number) => {
    await deviceApi.delete(id);
    refresh();
  };

  const refreshStatus = async () => {
    const results = await deviceApi.refreshStatus();
    setDevices(prev => prev.map(d => {
      const r = results.find(r => r.id === d.id);
      return r ? { ...d, status: r.status as Device['status'] } : d;
    }));
  };

  return { devices, loading, refresh, create, update, remove, refreshStatus };
}