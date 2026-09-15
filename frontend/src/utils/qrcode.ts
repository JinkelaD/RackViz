import type { Device } from '../types';

/**
 * ============================================================================
 * N-14 设备二维码内容编码（纯函数，无副作用，可单测）
 * ============================================================================
 *
 * ## 编码格式 v2（当前，稳定契约，勿随意变更）
 *
 * ```
 * RV|<设备ID>|<设备名称>|<型号>|<资产编号>|<序列号>
 * ```
 *
 * - 第 1 段恒为固定前缀 `RV`（RackViz）；其后依次为设备ID、设备名称、型号、资产编号、序列号。
 * - 共 **6 段固定**，段间以竖线 `|`（U+007C, `QR_FIELD_SEP`）分隔。
 * - **缺值仍占位**：任一文本字段为空串时保留空段，分隔符不会被"吃掉"，
 *   因此解析时字段位置恒定、无歧义。
 * - 内容为**明文可读字符串**（非 JSON、非 base64），便于对码人工核对与手机直扫。
 *
 * 示例：
 * ```
 * RV|208|B-02-存储阵列-01|华为 OceanStar 5310 V6|ZC-2026-0045|SN-B-02-006
 * RV|1024||||                                  // id=1024，其余四项均空
 * RV|7|核心交换机|Cisco 9300||                  // 资产编号、序列号为空
 * ```
 *
 * ## 编码格式 v1（历史，仅解析兼容）
 *
 * ```
 * RV|<设备ID>|<资产编号>|<序列号>          // 4 段，无 name/model
 * ```
 *
 * 早期打印的旧标签仍可被 {@link parseDeviceQr} 解析（`name`/`model` 返回空串），
 * 旧标签不会因格式升级而解析失败。
 *
 * ## 转义规则（防字段内混入分隔符造成解析歧义）
 *
 * **所有自由文本字段（名称 / 型号 / 资产编号 / 序列号）**都要转义——设备名称里完全可能含 `|`：
 * 1. 先 `\` → `\\`
 * 2. 再 `|` → `\|`
 *
 * 解析时按相反顺序还原。**分隔符选择依据**：现实资产编号 / 序列号为字母、数字、连字符
 * （见 T3.2 实测数据），本就不含 `|`；即便含，转义后仍可无损还原，双层保险。
 * 设备ID 为纯数字，无需转义。
 */

/** 固定前缀（版本无关的载荷标识） */
export const QR_PREFIX = 'RV';

/** 字段分隔符：竖线 U+007C */
export const QR_FIELD_SEP = '|';

/** v2 载荷固定段数（前缀 + id + 名称 + 型号 + 资产编号 + 序列号） */
export const QR_FIELD_COUNT = 6;

/** v1 载荷固定段数（前缀 + id + 资产编号 + 序列号），仅用于解析兼容 */
export const QR_FIELD_COUNT_V1 = 4;

/** 解析后的二维码载荷 */
export interface DeviceQrPayload {
  /** 设备ID；无法解析或为空时为 `null` */
  id: number | null;
  /** 设备名称（已还原转义）；v1 旧载荷或缺失时为 `''` */
  name: string;
  /** 型号名称（已还原转义）；v1 旧载荷或缺失时为 `''` */
  model: string;
  /** 资产编号（已还原转义）；缺失时为 `''` */
  asset_no: string;
  /** 序列号（已还原转义）；缺失时为 `''` */
  serial_no: string;
}

/** 参与编码的设备字段（`model` 为型号名称，可选/可空） */
export type QrDeviceInput =
  Pick<Device, 'id' | 'name' | 'asset_no' | 'serial_no'> & { model?: string | null };

/** 归一化：null/undefined → `''`，并去除首尾空白，保证空段稳定 */
function normalizeField(value: string | null | undefined): string {
  if (value == null) return '';
  return String(value).trim();
}

/**
 * 转义自由文本字段：`\` → `\\`，`|` → `\|`。
 * @param value 原始字段值
 * @returns 可安全嵌入分隔符串的转义值
 */
export function escapeQrField(value: string): string {
  return value.replace(/\\/g, '\\\\').replace(/\|/g, '\\|');
}

/**
 * 还原 {@link escapeQrField} 的转义。
 * @param value 转义后的字段值
 * @returns 原始字段值
 */
export function unescapeQrField(value: string): string {
  let out = '';
  for (let i = 0; i < value.length; i += 1) {
    const ch = value.charAt(i);
    if (ch === '\\') {
      const next = value.charAt(i + 1);
      if (next === '\\' || next === QR_FIELD_SEP) {
        out += next;
        i += 1;
        continue;
      }
    }
    out += ch;
  }
  return out;
}

/** 按**未转义**的 `|` 切分（跳过 `\|`、`\\` 序列），返回原始（仍含转义）字段数组 */
function splitQrFields(content: string): string[] {
  const fields: string[] = [];
  let current = '';
  for (let i = 0; i < content.length; i += 1) {
    const ch = content.charAt(i);
    if (ch === '\\') {
      const next = content.charAt(i + 1);
      if (next === '\\' || next === QR_FIELD_SEP) {
        current += ch + next;
        i += 1;
        continue;
      }
    }
    if (ch === QR_FIELD_SEP) {
      fields.push(current);
      current = '';
      continue;
    }
    current += ch;
  }
  fields.push(current);
  return fields;
}

/** 把 `parts[1]` 段（设备ID）解析为数字，非法/空返回 `null` */
function parseId(raw: string): number | null {
  if (raw === '') return null;
  const num = Number(raw);
  return Number.isFinite(num) ? num : null;
}

/**
 * 编码设备二维码内容（v2）。
 * @param device 至少包含 `id` / `name` / `asset_no` / `serial_no`，`model` 可选（型号名称）
 * @returns 形如 `RV|<id>|<name>|<model>|<asset_no>|<serial_no>` 的明文载荷
 */
export function encodeDeviceQr(device: QrDeviceInput): string {
  const id = Number.isFinite(device.id) ? String(device.id) : '';
  const name = escapeQrField(normalizeField(device.name));
  const model = escapeQrField(normalizeField(device.model));
  const assetNo = escapeQrField(normalizeField(device.asset_no));
  const serialNo = escapeQrField(normalizeField(device.serial_no));
  return [QR_PREFIX, id, name, model, assetNo, serialNo].join(QR_FIELD_SEP);
}

/**
 * 解析二维码内容（{@link encodeDeviceQr} 的逆函数），用于自检 / 往返测试 / 未来扫码入口。
 * 兼容 v2（6 段）与 v1（4 段）两种载荷。
 * @param content 二维码明文内容
 * @returns 解析结果；前缀或段数不符时返回 `null`
 */
export function parseDeviceQr(content: string): DeviceQrPayload | null {
  const parts = splitQrFields(content);
  if (parts[0] !== QR_PREFIX) return null;

  if (parts.length === QR_FIELD_COUNT) {
    // v2：RV | id | name | model | asset_no | serial_no
    return {
      id: parseId(parts[1]),
      name: unescapeQrField(parts[2]),
      model: unescapeQrField(parts[3]),
      asset_no: unescapeQrField(parts[4]),
      serial_no: unescapeQrField(parts[5]),
    };
  }

  if (parts.length === QR_FIELD_COUNT_V1) {
    // v1 旧载荷：RV | id | asset_no | serial_no（无 name/model）
    return {
      id: parseId(parts[1]),
      name: '',
      model: '',
      asset_no: unescapeQrField(parts[2]),
      serial_no: unescapeQrField(parts[3]),
    };
  }

  return null;
}
