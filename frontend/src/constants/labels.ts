import type { DeviceType } from '../types';

/**
 * 设备类型 → 元数据（唯一来源）。
 * 修改类型集合只需动这里；CSS 变量与渲染 class 已按此顺序约定：
 *   --type-<key> 变量、.type-<key> 设备块、.lib-icon.<key>、.detail-type-badge.<key>
 *   （CSS 中需为新 key 同步补规则，见 styles/global.css 的 "type-color-map" 区块）
 */
export const DEVICE_TYPES: DeviceType[] = [
  'server',
  'switch',
  'router',
  'storage',
  'nas',
  'security',
  'loadbalancer',
  'other',
];

export const DEVICE_TYPE_LABELS: Record<string, string> = {
  server: '服务器',
  switch: '交换机',
  router: '路由器',
  storage: '存储阵列',
  nas: 'NAS 存储',
  security: '网安设备',
  loadbalancer: '负载均衡',
  other: '其他',
};

/** antd Tag 配色（表格/型号管理弹窗） */
export const DEVICE_TYPE_TAG_COLORS: Record<string, string> = {
  server: 'blue',
  switch: 'cyan',
  router: 'gold',
  storage: 'purple',
  nas: 'green',
  security: 'volcano',
  loadbalancer: 'magenta',
  other: 'default',
};

/** 型号管理下拉选项 */
export const DEVICE_TYPE_OPTIONS = DEVICE_TYPES.map(t => ({
  value: t,
  label: DEVICE_TYPE_LABELS[t],
}));

const DEVICE_TYPE_SET = new Set<string>(DEVICE_TYPES);

function isDeviceType(v: string): v is DeviceType {
  return DEVICE_TYPE_SET.has(v);
}

/**
 * 归一化类型 → 已知类型或 'other'（兜底）。
 * 旧数据/未知 type 不允许落入无配色的裸 class。
 */
export function safeDeviceType(type?: string | null): DeviceType {
  return type && isDeviceType(type) ? type : 'other';
}

/** 设备状态 → 中文文本（唯一来源，多处复用禁止各自重复定义） */
export function getStatusText(status: string): string {
  if (status === 'online') return '开机';
  if (status === 'offline') return '离线';
  return '未上架';
}
