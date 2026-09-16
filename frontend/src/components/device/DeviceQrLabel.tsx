import { QRCodeSVG } from 'qrcode.react';
import { buildQrRows, encodeDeviceQr, QR_COLON } from '../../utils/qrcode';

interface DeviceQrLabelProps {
  /** 设备名称 */
  name: string;
  /** 设备型号名称（可选，仅用于标签文字） */
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
 * 在任意打印 DPI 下边缘清晰；canvas 为位图，高 DPI 打印会糊。设计所述"双模式适配打印"
 * 即指此选择。
 *
 * 标签结构：左侧二维码，右侧设备可读文字。**标签正文恒为 7 行**，顺序与二维码内容完全一致
 * （`设备名称 / 型号 / IP地址 / 资产编号 / 序列号 / 使用部门 / 责任人`），且由同一份 `buildQrRows`
 * 数据渲染，保证"印着的内容"与"扫到的内容"逐字一致。空字段仍保留整行（如 `使用部门：`），
 * 文本允许换行、不截断（窄面板下标签变高而非省略信息），二维码尺寸固定以保证可扫率。
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
  const rows = buildQrRows({
    name,
    model: modelName ?? null,
    ip_addresses: ipAddresses,
    asset_no: assetNo,
    serial_no: serialNo,
    department,
    owner,
  });
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
      <div className="qr-label-head">
        <span className="qr-label-brand">RackViz</span>
        <span className="qr-label-title">设备标签</span>
      </div>

      <div className="qr-label-body">
        <div className="qr-label-code">
          <QRCodeSVG
            value={content}
            size={QR_SIZE}
            level="M"
            marginSize={QR_QUIET_ZONE}
            title={`设备二维码：${name}`}
          />
        </div>

        <div className="qr-label-info">
          {rows.map((row) => (
            <div className="qr-label-row" key={row.key}>
              <span className="qr-label-key">
                {row.key}
                {QR_COLON}
              </span>
              <span className="qr-label-val" title={row.value}>
                {row.value}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
