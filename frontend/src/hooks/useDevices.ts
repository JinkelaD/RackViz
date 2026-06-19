import { Device } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

export function useDevices() {
  const { items: devices, loading, refresh, create, update, remove } = useApiList<Device>(
    () => api.listDevices() as Promise<Device[]>,
    (data) => api.createDevice(data as api.DeviceCreate) as Promise<Device>,
    (id, data) => api.updateDevice(id, data as api.DeviceUpdate) as Promise<Device | null>,
    (id) => api.deleteDevice(id),
    { optimistic: true },
  );
  return { devices, loading, refresh, create, update, remove };
}
