import { invoke } from '@tauri-apps/api/core';
import type { Device, DeviceModel, Rack, Room } from './types';

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

export interface ImportResult {
  imported: number;
  skipped: number;
  errors: string[];
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

export function listDevices(rackId?: number, search?: string): Promise<DeviceResp[]> {
  return invoke('list_devices', { rackId: rackId ?? null, search: search ?? null });
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
export function deleteDevice(id: number): Promise<boolean> {
  return invoke('delete_device', { id });
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
// 导入由 Rust 侧弹出原生打开对话框并直接读文件

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

/** 导入 Excel — Rust 侧弹出原生文件打开对话框 */
export async function importExcelFromPath(): Promise<ImportResult> {
  try {
    // 使用 Tauri 的 dialog 插件打开文件选择
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      filters: [{ name: 'Excel 文件', extensions: ['xlsx', 'xls'] }],
      multiple: false,
    });
    if (!selected) {
      return { imported: 0, skipped: 0, errors: [] };
    }
    // Tauri 2.x dialog open returns string path or { path: string }
    const filePath = typeof selected === 'string' ? selected : (selected as { path: string }).path;
    return invoke('import_excel_from_path', { path: filePath });
  } catch (err) {
    console.error('导入失败:', err);
    return { imported: 0, skipped: 0, errors: [String(err)] };
  }
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
