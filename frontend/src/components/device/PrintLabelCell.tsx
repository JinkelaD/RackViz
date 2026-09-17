import { QRCodeSVG } from 'qrcode.react';
import { encodeDeviceQr } from '../../utils/qrcode';
import type { Device } from '../../types';

/** 二维码边长（px），屏幕预览用；打印时由 CSS 矢量覆盖为 22mm */
const QR_SIZE = 132;

/**
 * 安静区（二维码四周的空白边框），单位为**模块数**。
 *
 * 必须为 4：ISO/IEC 18004 规定安静区为 4 模块，`qrcode.react` 的类型定义亦明确
 * "The QR Code specification requires 4"（其 `marginSize` 默认值为 0）。
 * 安静区不足时，标签边缘的裁切误差/污损会直接吃掉定位图案，导致打印后扫不出。
 */
const QR_QUIET_ZONE = 4;

interface PrintLabelCellProps {
  device: Device;
  /** 机柜显示名；未上架时为 null（文字区显示「未上架」） */
  rackName: string | null;
  /** 型号名称（仅用于二维码内容） */
  modelName?: string | null;
}

/**
 * 批量打印标签单元（B4 重构版）：左码右文。
 *
 * - 二维码内容沿用 7 字段中文明文契约（`encodeDeviceQr`），与旧单码标签完全同源；
 * - 文字区为契约的可视化摘要（设备名 / 机柜-U位 / 资产编号 / SN），
 *   解决批量打印后标签外观相似易贴错的问题——人眼即可区分；
 * - 必须使用 SVG 模式（`QRCodeSVG`）：打印为矢量输出，任意 DPI 边缘清晰。
 */
export default function PrintLabelCell({ device, rackName, modelName }: PrintLabelCellProps) {
  const content = encodeDeviceQr({
    name: device.name,
    model: modelName ?? null,
    ip_addresses: device.ip_addresses,
    asset_no: device.asset_no,
    serial_no: device.serial_no,
    department: device.department,
    owner: device.owner,
  });

  const uRange =
    device.start_u != null && device.end_u != null ? `U${device.start_u}-U${device.end_u}` : null;

  return (
    <div className="print-label-cell">
      <div className="print-label-code">
        <QRCodeSVG
          value={content}
          size={QR_SIZE}
          level="M"
          marginSize={QR_QUIET_ZONE}
          title={`设备二维码：${device.name}`}
        />
      </div>
      <div className="print-label-text">
        <span className="print-label-name" title={device.name}>{device.name}</span>
        <span className="print-label-loc">
          {rackName ? `${rackName} · ${uRange ?? '未定 U 位'}` : '未上架'}
        </span>
        <span className="print-label-asset">{device.asset_no || '-'}</span>
        <span className="print-label-sn" title={device.serial_no}>{device.serial_no || '-'}</span>
      </div>
    </div>
  );
}
