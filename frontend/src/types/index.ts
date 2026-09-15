/**
 * 前端类型唯一来源（single source of truth）。
 *
 * 约定（见 v2.0.0 架构设计 §8）：
 * - 所有实体 / 负载 / DTO 类型只定义在本文件；`tauri-api.ts` 仅做 `Partial<T> & { name }` 派生。
 * - 与 Rust 交互的 DTO 一律 snake_case（N-A 裁决），字段名不得改写为 camelCase。
 * - 时间戳（created_at / updated_at）由后端维护，前端只读；deleted_at 仅设备（N-09 软删除）。
 */

export interface DeviceModel {
  id: number;
  name: string;
  manufacturer: string;
  type: DeviceType;
  height_u: number;
  power_watt: number;
  created_at: string | null;
  updated_at: string | null;
}

export type DeviceType = 'server' | 'switch' | 'router' | 'storage' | 'nas' | 'security' | 'loadbalancer' | 'other';

export interface Room {
  id: number;
  name: string;
  location: string;
  sort_order: number;
  created_at: string | null;
  updated_at: string | null;
}

export interface Rack {
  id: number;
  name: string;
  height_u: number;
  row: number;
  col: number;
  view: 'front' | 'rear';
  sort_order: number;
  room_id: number | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface Device {
  id: number;
  name: string;
  device_model_id: number | null;
  rack_id: number | null;
  start_u: number | null;
  end_u: number | null;
  ip_addresses: string;
  serial_no: string;
  asset_no: string;
  department: string;
  owner: string;
  function: string;
  purchase_date: string | null;
  warranty_expire: string | null;
  status: 'online' | 'offline' | 'unconfigured';
  power_watt: number;
  /** 设备固有高度（U）；下架后保留，重上架不再退化为 1U */
  height_u: number;
  /** 落库时间戳（后端维护，前端只读）；历史数据迁移时回填为迁移时刻 */
  created_at: string | null;
  updated_at: string | null;
  /** N-09 软删除标记；非空表示已删除，可在 30 天内于回收站恢复 */
  deleted_at: string | null;
}

export type EditMode = 'drag' | 'delete' | 'edit';
export type ViewMode = 'front' | 'rear';

// ============================================================================
// Stage 2A：服务端分页 / 软删除 / 批量删除 / 备份恢复 / 导入（N-01/N-04/N-06/N-09/N-18/N-20）
// —— 全部 DTO 保持 snake_case（N-A 裁决）
// ============================================================================

/** 服务端排序白名单字段（与后端 `query_devices` 白名单一一对应） */
export type DeviceSortField =
  | 'name'
  | 'ip_addresses'
  | 'serial_no'
  | 'asset_no'
  | 'status'
  | 'power_watt'
  | 'created_at'
  | 'updated_at'
  | 'rack_id';

/** 服务端分页/搜索/排序查询负载（N-01/N-02） */
export interface DeviceQuery {
  rack_id?: number | null;
  room_id?: number | null;
  search?: string | null;
  include_deleted?: boolean;
  sort_field?: DeviceSortField | null;
  sort_order?: 'asc' | 'desc' | null;
  offset?: number | null;
  limit?: number | null;
}

/** 分页结果（与后端 `list_devices` 返回一致） */
export interface DevicePage {
  items: Device[];
  total: number;
}

/** Excel 导入选项（N-04/N-05） */
export interface ImportOptions {
  /** skip = 已存在则跳过；overwrite = 按查重键覆盖字段并保留主键 */
  update_mode: 'skip' | 'overwrite';
  /** 是否依据导入文件的机房列自动创建/关联机房（幂等） */
  link_room: boolean;
}

/**
 * Excel 导入结果（N-04/N-06/N-21）。
 * 由 `tauri-api.ts` 迁入本文件，满足「类型单一来源」约定。
 */
export interface ImportResult {
  imported: number;
  updated: number;
  skipped: number;
  total: number;
  /** N-21 本次新建的型号数量 */
  models_created: number;
  /** N-21 告警项（未知/空设备类型兜底 other、同名型号复用等） */
  warnings: string[];
  errors: string[];
}

/** N-20 批量删除结果（snake_case） */
export interface DeleteBatchResult {
  deleted: number;
  /** 不存在或已被软删、被跳过的 id 列表 */
  not_found: number[];
}

/** N-18 数据库恢复结果 */
export interface RestoreResult {
  restart_required: boolean;
  message: string;
}

/** N-06 导入进度事件 payload（事件名 `import://progress`） */
export interface ImportProgress {
  processed: number;
  total: number;
  phase: 'parsing' | 'importing' | 'done';
}

/** N-15 主题预设（精简为 2 套）：`night`=夜间蓝（暗色/默认）、`eye`=护眼绿（亮色） */
export type ThemePreset = 'night' | 'eye';
