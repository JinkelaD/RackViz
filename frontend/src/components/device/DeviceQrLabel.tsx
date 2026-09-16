import { QRCodeSVG } from 'qrcode.react';
import { encodeDeviceQr } from '../../utils/qrcode';

interface DeviceQrLabelProps {
  /** 设备名称 */
  name: string;
  /** 设备型号名称（可选，仅用于二维码内容） */
  modelName?: string | null;
  /** IP 地址（`;` 分隔，原样输出） */
  ipAddresses: string;
  /** 资产编号 */
  assetNo: string;
  /** 序列号 */
  serialNo: string;
  /** 使用部门 */
  department: string;
  /** 责任人 */
  owner: string;
}

/** 二维码边长（px）；SVG 为矢量，打印时按矢量缩放不糊 */
const QR_SIZE = 132;

/**
 * 安静区（二维码四周的空白边框），单位为**模块数**。
 *
 * 必须为 4：ISO/IEC 18004 规定安静区为 4 模块，`qrcode.react` 的类型定义亦明确
 * "The QR Code specification requires 4"（其 `marginSize` 默认值为 0）。
 * 安静区不足时，标签边缘的裁切误差/污损会直接吃掉定位图案，导致打印后扫不出。
 *
 * 注：`size` 已包含安静区，故改为 4 不会撑大布局，仅使符号本身在 132px 内略微缩小
 * （v10/53 模块下每模块约 0.57mm，仍远高于 0.4mm 的可扫下限）。
 */
const QR_QUIET_ZONE = 4;

/**
 * 设备二维码标签。
 *
 * **必须使用 SVG 模式（`QRCodeSVG`）而非 canvas**：标签用于打印，SVG 是矢量输出，
 * 在任意打印 DPI 下边缘清晰；canvas 为位图，高 DPI 打印会糊。
 *
 * 标签结构（2026-09-16 主理人指示调整）：**仅显示二维码图片**，去除品牌行与全部可读文字，
 * 保证二维码以最大可用面积清晰完整、便于扫描。设备信息全部承载于二维码内容本身
 * （恒为 7 行，顺序固定：`设备名称 / 型号 / IP地址 / 资产编号 / 序列号 / 使用部门 / 责任人`，
 * 由 `encodeDeviceQr` 生成），扫码即得完整信息。
 */
export default function DeviceQrLabel({
  name,
  modelName,
  ipAddresses,
  assetNo,
  serialNo,
  department,
  owner,
}: DeviceQrLabelProps) {
  const content = encodeDeviceQr({
    name,
    model: modelName ?? null,
    ip_addresses: ipAddresses,
    asset_no: assetNo,
    serial_no: serialNo,
    department,
    owner,
  });

  return (
    <div className="qr-label-sheet">
      <div className="qr-label-code">
        <QRCodeSVG
          value={content}
          size={QR_SIZE}
          level="M"
          marginSize={QR_QUIET_ZONE}
          title={`设备二维码：${name}`}
        />
      </div>
    </div>
  );
}
