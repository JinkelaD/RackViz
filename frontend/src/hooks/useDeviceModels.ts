import { useState, useEffect, useCallback } from 'react';
import { DeviceModel } from '../types';
import { deviceModelApi } from '../api/client';

export function useDeviceModels() {
  const [models, setModels] = useState<DeviceModel[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(() => {
    setLoading(true);
    deviceModelApi.list().then(setModels).finally(() => setLoading(false));
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  const create = async (data: Partial<DeviceModel>) => {
    await deviceModelApi.create(data);
    refresh();
  };

  const update = async (id: number, data: Partial<DeviceModel>) => {
    await deviceModelApi.update(id, data);
    refresh();
  };

  const remove = async (id: number) => {
    await deviceModelApi.delete(id);
    refresh();
  };

  return { models, loading, refresh, create, update, remove };
}