import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  DeleteBatchResult,
  Device,
  DeviceModel,
  DevicePage,
  DeviceQuery,
  ImportOptions,
  ImportProgress,
  ImportResult,
  Rack,
  RestoreResult,
  Room,
} from './types';

// ===== 类型定义（单一来源：均从 types/index.ts 派生，避免双份漂移）=====

// 响应/实体类型 = domain 类型
export type DeviceResp = Device;
export type RackResp = Rack;
export type RoomResp = Room;
export type ModelResp = DeviceModel;

// 创建/更新负载：基于 domain 类型的 Partial（字段名/可空性自动同步）
// 注意：可置空外键（rack_id/start_u/end_u/room_id…）允许 null，对应后端 Patch::Clear
export type DeviceCreate = Partial<Device> & { name: string };
export type DeviceUpdate = Partial<Device>;
export type RackCreate = Partial<Rack> & { name: string };
export type RackUpdate = Partial<Rack>;
export type RoomCreate = Partial<Room> & { name: string };
export type RoomUpdate = Partial<Room>;
export type ModelCreate = Partial<DeviceModel> & { name: string };
export type ModelUpdate = Partial<DeviceModel>;

// ===== 工具：统一错误文案提取 =====
/**
 * 从 Tauri invoke 的拒绝值中提取可读错误文案。
 * 后端 AppError 通常序列化为字符串（如「序列号已被设备「Y」占用」），
 * 也可能是 Error 对象或 { message } 结构，这里统一兜底，避免出现 "[object Object]"。
 */
export function errorMessage(err: unknown): string {
  if (typeof err === 'string') return err;
  if (err instanceof Error) return err.message;
  if (err && typeof err === 'object' && 'message' in err) {
    const m = (err as { message?: unknown }).message;
    if (typeof m === 'string') return m;
  }
  try {
    return JSON.stringify(err);
  } catch {
    return '未知错误';
  }
}

// ===== Room API =====

export function listRooms(): Promise<RoomResp[]> {
  return invoke('list_rooms');
}
export function getRoom(id: number): Promise<RoomResp | null> {
  return invoke('get_room', { id });
}
export function createRoom(data: RoomCreate): Promise<RoomResp> {
  return invoke('create_room', { data });
}
export function updateRoom(id: number, data: RoomUpdate): Promise<RoomResp | null> {
  return invoke('update_room', { id, data });
}
export function deleteRoom(id: number): Promise<boolean> {
  return invoke('delete_room', { id });
}

// ===== Rack API =====

export function listRacks(): Promise<RackResp[]> {
  return invoke('list_racks');
}
export function getRack(id: number): Promise<RackResp | null> {
  return invoke('get_rack', { id });
}
export function createRack(data: RackCreate): Promise<RackResp> {
  return invoke('create_rack', { data });
}
export function updateRack(id: number, data: RackUpdate): Promise<RackResp | null> {
  return invoke('update_rack', { id, data });
}
export function deleteRack(id: number): Promise<boolean> {
  return invoke('delete_rack', { id });
}

// ===== Device API =====

/** 服务端分页 + 多字段搜索（N-01/N-02）；`list_devices` 已改造为接收 `query` 的分页版 */
export function queryDevices(query: DeviceQuery): Promise<DevicePage> {
  return invoke('list_devices', { query });
}

/** 全量设备（供 RackView / 导出 / 报表使用）；恒过滤软删记录 */
export function listDevicesAll(): Promise<DeviceResp[]> {
  return invoke('list_devices_all');
}

/** 回收站：已软删除的设备列表（N-09） */
export function listDeletedDevices(): Promise<DeviceResp[]> {
  return invoke('list_deleted_devices');
}

/** 恢复已软删除设备（N-09）；恢复失败（序列号/资产编号冲突、超 30 天）时 reject */
export function restoreDevice(id: number): Promise<DeviceResp> {
  return invoke('restore_device', { id });
}

export function getDevice(id: number): Promise<DeviceResp | null> {
  return invoke('get_device', { id });
}
export function createDevice(data: DeviceCreate): Promise<DeviceResp> {
  return invoke('create_device', { data });
}
export function updateDevice(id: number, data: DeviceUpdate): Promise<DeviceResp | null> {
  return invoke('update_device', { id, data });
}
/** 单条删除（语义已改为软删，N-09） */
export function deleteDevice(id: number): Promise<boolean> {
  return invoke('delete_device', { id });
}

/**
 * N-20 批量删除：仅发起 1 次 IPC（禁止前端循环调单条）。
 * 后端在单事务内复用单条软删函数，任一失败整体回滚。
 */
export function deleteDevices(ids: number[]): Promise<DeleteBatchResult> {
  return invoke('delete_devices', { ids });
}

// ===== Device Model API =====

export function listDeviceModels(): Promise<ModelResp[]> {
  return invoke('list_device_models');
}
export function getDeviceModel(id: number): Promise<ModelResp | null> {
  return invoke('get_device_model', { id });
}
export function createDeviceModel(data: ModelCreate): Promise<ModelResp> {
  return invoke('create_device_model', { data });
}
export function updateDeviceModel(id: number, data: ModelUpdate): Promise<ModelResp | null> {
  return invoke('update_device_model', { id, data });
}
export function deleteDeviceModel(id: number): Promise<boolean> {
  return invoke('delete_device_model', { id });
}

// ===== Export/Import API =====
// 注意：导出由 Rust 侧弹出原生保存对话框并直接写文件

export function exportRacksExcel(): Promise<string> {
  return invoke('export_racks_excel');
}
export function exportDevicesDataExcel(): Promise<string> {
  return invoke('export_devices_data_excel');
}
export function exportSingleRackExcel(rackId: number): Promise<string> {
  return invoke('export_single_rack_excel', { rackId });
}
export function exportReportHtml(): Promise<string> {
  return invoke('export_report_html');
}

/**
 * 导入 Excel（N-04/N-05/N-06/N-21）。
 * 由导入向导先行选择文件、收集选项，再调用本函数；进度经 `import://progress` 事件推送。
 * @param path 已选定的 xlsx/xls 绝对路径
 * @param options 更新模式（skip/overwrite）与是否自动关联机房
 */
export function importExcelFromPath(path: string, options: ImportOptions): Promise<ImportResult> {
  return invoke('import_excel_from_path', { path, options });
}

/**
 * 订阅导入进度事件（N-06）。调用方必须在导入结束后（`finally`）调用返回的 unlisten。
 */
export function onImportProgress(cb: (p: ImportProgress) => void): Promise<UnlistenFn> {
  return listen<ImportProgress>('import://progress', (e) => cb(e.payload));
}

// ===== 维护 API（N-18 备份 / 恢复）=====

/** 备份数据库（Rust 侧弹原生保存对话框，返回备份文件路径） */
export function backupDatabase(): Promise<string> {
  return invoke('backup_database');
}

/** 从备份恢复（延迟交换 + 重启生效） */
export function restoreDatabase(path: string): Promise<RestoreResult> {
  return invoke('restore_database', { path });
}

// ===== Settings API =====

export interface LoggingConfig {
  enabled: boolean;
  logDir: string;
}

export function getLoggingConfig(): Promise<LoggingConfig> {
  return invoke('get_logging_config');
}

export function setLoggingEnabled(enabled: boolean): Promise<LoggingConfig> {
  return invoke('set_logging_enabled', { enabled });
}

export function openLogDir(): Promise<void> {
  return invoke('open_log_dir');
}
