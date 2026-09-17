/**
 * 前端类型唯一来源（single source of truth）。
 *
 * 约定（见 v2.0.0 架构设计 §8；C1 自 ts-rs 接管 IPC 类型）：
 * - **IPC 类型**（Rust 侧 serde struct/enum 镜像）：由 `src-tauri/src/models.rs` 的
 *   `#[derive(ts_rs::TS)]` 在 `cargo test export` 时生成到 `./generated/`，本文件
 *   仅做 re-export —— **禁止手写与 generated 同名的类型**（防漂移）；
 *   生成物注释带 "Do not edit manually"，更新方式：改 Rust 结构体 → `cargo test export`。
 * - **纯前端类型**（不参与 IPC 序列化：UI 枚举/视图状态等）仍手写于本文件。
 * - 与 Rust 交互的 DTO 一律 snake_case（N-A 裁决）。
 * - 时间戳（created_at / updated_at）由后端维护，前端只读；deleted_at 仅设备（N-09 软删除）。
 * - 注意：Rust 侧 `status`/`view`/`type` 为 String（生成即 `string`）；枚举值约束由
 *   `constants/labels.ts` 的 DEVICE_TYPE_LABELS / safeDeviceType 等运行时守卫承担。
 */

// ===== IPC 类型（ts-rs 生成，勿手改）=====
export type { DeviceModel } from './generated/DeviceModel';
export type { Room } from './generated/Room';
export type { Rack } from './generated/Rack';
export type { Device } from './generated/Device';
export type { DeviceQuery } from './generated/DeviceQuery';
export type { DevicePage } from './generated/DevicePage';
export type { ImportOptions } from './generated/ImportOptions';
export type { ImportResult } from './generated/ImportResult';
export type { ImportProgress } from './generated/ImportProgress';
export type { DeleteBatchResult } from './generated/DeleteBatchResult';
export type { RestoreResult } from './generated/RestoreResult';

// ===== 纯前端类型（手写）=====

/** 设备类型 UI 枚举（与后端 normalize_device_type 的 8 个英文枚举值双源同步，§8-14） */
export type DeviceType = 'server' | 'switch' | 'router' | 'storage' | 'nas' | 'security' | 'loadbalancer' | 'other';

export type EditMode = 'drag' | 'delete' | 'edit';
export type ViewMode = 'front' | 'rear';

/** 服务端排序白名单字段（发送侧约束：UI 只会发出这些值；传输层为 string） */
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

/** N-15 主题预设（精简为 2 套）：`night`=夜间蓝（暗色/默认）、`eye`=护眼绿（亮色） */
export type ThemePreset = 'night' | 'eye';
