import { DeviceModel } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

export function useDeviceModels() {
  const { items: models, loading, refresh, create, update, remove } = useApiList<DeviceModel>(
    () => api.listDeviceModels() as Promise<DeviceModel[]>,
    (data) => api.createDeviceModel(data as api.ModelCreate) as Promise<DeviceModel>,
    (id, data) => api.updateDeviceModel(id, data as api.ModelUpdate) as Promise<DeviceModel | null>,
    (id) => api.deleteDeviceModel(id),
  );
  return { models, loading, refresh, create, update, remove };
}
