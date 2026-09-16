import type { Device } from '../types';

/**
 * ============================================================================
 * 设备二维码内容编码（纯函数，无副作用，可单测）
 * ============================================================================
 *
 * ## 编码格式（稳定契约，勿随意变更）
 *
 * 固定 7 个字段，每行 `字段名：字段值`，字段间以换行分隔：
 *
 * ```
 * 设备名称：A-01-存储阵列-01
 * 型号：Dell EMC Unity XT 480
 * IP地址：10.10.1.16;10.10.1.17
 * 资产编号：ZC-2026-0006
 * 序列号：SN-B-02-006
 * 使用部门：研发中心
 * 责任人：孙鹏
 * ```
 *
 * - **固定 7 行，顺序不可变**：设备名称 → 型号 → IP地址 → 资产编号 → 序列号 → 使用部门 → 责任人。
 * - **冒号为全角 `：`**（U+FF1A），与用户给定格式一致。
 * - **缺值仍占位**：任一字段为空串时保留整行（如 `使用部门：`），避免字段缺失导致行错位。
 * - 内容为**明文可读多行文本**（非 JSON、非 base64、无前缀、无分隔符），手机直扫即可阅读。
 * - **IP 地址多个用半角分号 `;` 分隔**：`Device.ip_addresses` 本就是 `;` 分隔，原样输出、不改写。
 *
 * ## 健壮性（防"换行错乱"）
 *
 * 字段值若含 `\r` / `\n`（脏数据），编码前**先替换为空格**再拼接；否则一行会被劈成两行、
 * 破坏"每字段一行"的结构。所有字段统一 `trim`。
 *
 * ## 解析
 *
 * `parseDeviceQr` 为 {@link encodeDeviceQr} 的逆函数，用于自检 / 往返测试 / 未来扫码入口。
 * 按换行 split → 每行按**首个**全角 `：` 切分 → 按字段名映射回结构化对象。
 * **行序或字段名任一不符、行数不为 7 时返回 `null`**（不做兼容、不猜测修复）。
 */

/** 字段名固定顺序（全角冒号 `：` 连接值）；数组即契约，改动需同步编码/解析/标签渲染 */
export const QR_FIELD_KEYS = [
  '设备名称',
  '型号',
  'IP地址',
  '资产编号',
  '序列号',
  '使用部门',
  '责任人',
] as const;

/** 字段分隔用的全角冒号（U+FF1A），编码与解析统一使用 */
export const QR_COLON = '：';

/** 参与编码的设备字段（`model` 为型号名称，可选/可空） */
export type QrDeviceInput =
  Pick<Device, 'name' | 'asset_no' | 'ip_addresses' | 'serial_no' | 'department' | 'owner'> & {
    model?: string | null;
  };

/** 解析后的二维码载荷（与 {@link QR_FIELD_KEYS} 顺序一一对应） */
export interface DeviceQrPayload {
  /** 设备名称（已归一化） */
  name: string;
  /** 型号名称（已归一化） */
  model: string;
  /** IP 地址（`;` 分隔，原样保留） */
  ip_addresses: string;
  /** 资产编号 */
  asset_no: string;
  /** 序列号 */
  serial_no: string;
  /** 使用部门 */
  department: string;
  /** 责任人 */
  owner: string;
}

/** 单行键值 */
export interface QrRow {
  key: string;
  value: string;
}

/**
 * 归一化字段值：null/undefined → `''`；去除首尾空白；将值内的 `\r`/`\n` 替换为空格，
 * 防止脏数据把一行劈成两行、破坏"每字段一行"结构。
 */
function normalizeField(value: string | null | undefined): string {
  if (value == null) return '';
  return String(value)
    .replace(/[\r\n]+/g, ' ')
    .trim();
}

/**
 * 构造编码所需的 7 行键值（已归一化，顺序固定）。
 * 编码（{@link encodeDeviceQr}）与标签渲染共用此函数，保证"印着的内容"与"扫到的内容"逐字一致。
 */
export function buildQrRows(input: QrDeviceInput): QrRow[] {
  return [
    { key: '设备名称', value: normalizeField(input.name) },
    { key: '型号', value: normalizeField(input.model) },
    { key: 'IP地址', value: normalizeField(input.ip_addresses) },
    { key: '资产编号', value: normalizeField(input.asset_no) },
    { key: '序列号', value: normalizeField(input.serial_no) },
    { key: '使用部门', value: normalizeField(input.department) },
    { key: '责任人', value: normalizeField(input.owner) },
  ];
}

/**
 * 编码设备二维码内容。
 * @param input 设备字段（`name`/`asset_no`/`ip_addresses`/`department`/`owner` 必填，`model` 可选）
 * @returns 形如 `设备名称：xx\n型号：xx\n...` 的 7 行明文载荷
 */
export function encodeDeviceQr(input: QrDeviceInput): string {
  return buildQrRows(input)
    .map((row) => `${row.key}${QR_COLON}${row.value}`)
    .join('\n');
}

/**
 * 解析二维码内容（{@link encodeDeviceQr} 的逆函数），用于自检 / 往返测试 / 未来扫码入口。
 * @param content 二维码明文内容
 * @returns 解析结果；行数不为 7、行序错乱或字段名未知时返回 `null`
 */
export function parseDeviceQr(content: string): DeviceQrPayload | null {
  if (typeof content !== 'string') return null;

  const lines = content.split(/\r?\n/);
  if (lines.length !== QR_FIELD_KEYS.length) return null;

  const result: Record<string, string> = {};
  for (let i = 0; i < QR_FIELD_KEYS.length; i += 1) {
    const line = lines[i];
    const sep = line.indexOf(QR_COLON);
    if (sep < 0) return null; // 无全角冒号，结构不对
    const key = line.slice(0, sep);
    const value = line.slice(sep + 1);
    if (key !== QR_FIELD_KEYS[i]) return null; // 字段名/行序不符
    result[key] = value;
  }

  return {
    name: result['设备名称'],
    model: result['型号'],
    ip_addresses: result['IP地址'],
    asset_no: result['资产编号'],
    serial_no: result['序列号'],
    department: result['使用部门'],
    owner: result['责任人'],
  };
}
