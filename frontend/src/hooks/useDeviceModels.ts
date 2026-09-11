import { DeviceModel } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

export function useDeviceModels() {
  const { items: models, loading, refresh, create, update, remove } = useApiList<DeviceModel>(
    () => api.listDeviceModels(),
    (data) => api.createDeviceModel(data as api.ModelCreate),
    (id, data) => api.updateDeviceModel(id, data),
    (id) => api.deleteDeviceModel(id),
  );
  return { models, loading, refresh, create, update, remove };
}
