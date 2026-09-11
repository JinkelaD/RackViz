import { Device } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

export function useDevices() {
  const { items: devices, loading, refresh, create, update, remove } = useApiList<Device>(
    () => api.listDevices(),
    (data) => api.createDevice(data as api.DeviceCreate),
    (id, data) => api.updateDevice(id, data),
    (id) => api.deleteDevice(id),
    { optimistic: true },
  );
  return { devices, loading, refresh, create, update, remove };
}
