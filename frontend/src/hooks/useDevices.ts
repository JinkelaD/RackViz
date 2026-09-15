import { useCallback } from 'react';
import { Device, DeleteBatchResult } from '../types';
import * as api from '../tauri-api';
import { useApiList } from './useApiList';

/**
 * 设备数据 Hook。
 * - 列表走全量命令 `listDevicesAll`（RackView / 台账共用；服务端分页由 T3.2 的 usePagedDevices 负责）。
 * - 单条删除保持既有乐观更新（useApiList 选项 optimistic:true）。
 * - N-20 批量删除走**非乐观**路径：`await` 成功后整体 `refresh()`，失败时本地列表不变并向上抛出，
 *   不接入 useApiList 的乐观语义（批量回滚 UI 复杂度不划算），以保持 useApiList 语义纯净。
 */
export function useDevices() {
  const { items: devices, loading, refresh, create, update, remove } = useApiList<Device>(
    () => api.listDevicesAll(),
    (data) => api.createDevice(data as api.DeviceCreate),
    (id, data) => api.updateDevice(id, data),
    (id) => api.deleteDevice(id),
    { optimistic: true },
  );

  /** N-20 批量删除（非乐观）：成功 refresh、失败抛出且列表不变 */
  const removeMany = useCallback(async (ids: number[]): Promise<DeleteBatchResult> => {
    const result = await api.deleteDevices(ids);
    refresh();
    return result;
  }, [refresh]);

  return { devices, loading, refresh, create, update, remove, removeMany };
}
