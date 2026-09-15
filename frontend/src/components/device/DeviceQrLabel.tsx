import { QRCodeSVG } from 'qrcode.react';
import type { Device } from '../../types';
import { encodeDeviceQr } from '../../utils/qrcode';

interface DeviceQrLabelProps {
  /** 待打印标签的设备 */
  device: Device;
  /** 设备型号名称（可选，仅用于标签文字） */
  modelName?: string;
}

/** 二维码边长（px）；SVG 为矢量，打印时按矢量缩放不糊 */
const QR_SIZE = 132;

/**
 * N-14 设备二维码标签。
 *
 * **必须使用 SVG 模式（`QRCodeSVG`）而非 canvas**：标签用于打印，SVG 是矢量输出，
 * 在任意打印 DPI 下边缘清晰；canvas 为位图，高 DPI 打印会糊。设计所述"双模式适配打印"
 * 即指此选择。
 *
 * 标签结构：左侧二维码，右侧设备可读文字（名称/型号/资产编号/序列号/设备ID）；
 * 文本允许换行、不截断（窄面板下标签变高而非省略信息），二维码尺寸固定以保证可扫率。
 */
export default function DeviceQrLabel({ device, modelName }: DeviceQrLabelProps) {
  const content = encodeDeviceQr({
    id: device.id,
    name: device.name,
    asset_no: device.asset_no,
    serial_no: device.serial_no,
    model: modelName,
  });
  const assetNo = device.asset_no?.trim() || '-';
  const serialNo = device.serial_no?.trim() || '-';

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
            marginSize={2}
            title={`设备二维码：${device.name}`}
          />
        </div>

        <div className="qr-label-info">
          <div className="qr-label-name" title={device.name}>{device.name}</div>
          <div className="qr-label-row">
            <span className="qr-label-key">型号</span>
            <span className="qr-label-val" title={modelName || '-'}>{modelName || '-'}</span>
          </div>
          <div className="qr-label-row">
            <span className="qr-label-key">资产编号</span>
            <span className="qr-label-val qr-label-mono" title={assetNo}>{assetNo}</span>
          </div>
          <div className="qr-label-row">
            <span className="qr-label-key">序列号</span>
            <span className="qr-label-val qr-label-mono" title={serialNo}>{serialNo}</span>
          </div>
          <div className="qr-label-row">
            <span className="qr-label-key">设备ID</span>
            <span className="qr-label-val qr-label-mono">{device.id}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
