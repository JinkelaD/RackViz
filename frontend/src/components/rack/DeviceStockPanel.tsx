import type { Device, DeviceModel, Rack } from '../../types';
import { DEVICE_TYPE_LABELS, safeDeviceType } from '../../constants/labels';
import DeviceDetailPanel from '../DeviceDetailPanel';

export interface SelectedDeviceInfo {
  device: Device;
  model: DeviceModel | undefined;
  rack: Rack | undefined;
}

interface DeviceStockPanelProps {
  unassignedDevices: Device[];
  draggingDeviceId: number | null;
  selectedDeviceId: number | null;
  dragOverStock: boolean;
  selectedDeviceInfo: SelectedDeviceInfo | null;
  getDeviceModel: (id: number | null) => DeviceModel | undefined;
  onDragStart: (e: React.DragEvent, device: Device) => void;
  onDragEnd: () => void;
  onStockDragOver: (e: React.DragEvent) => void;
  onStockDrop: (e: React.DragEvent) => void;
  onDeviceClick: (device: Device) => void;
  onDeviceDoubleClick: (device: Device) => void;
  onCloseDetail: () => void;
  onUpdateDetail: (id: number, data: Partial<Device>) => Promise<void>;
  onRemoveDetail: (id: number) => Promise<void>;
}

/** 右侧栏：资源池（未上架设备）+ 设备详情面板 */
export default function DeviceStockPanel(props: DeviceStockPanelProps) {
  const {
    unassignedDevices, draggingDeviceId, selectedDeviceId, dragOverStock, selectedDeviceInfo,
    getDeviceModel,
    onDragStart, onDragEnd, onStockDragOver, onStockDrop,
    onDeviceClick, onDeviceDoubleClick,
    onCloseDetail, onUpdateDetail, onRemoveDetail,
  } = props;

  return (
    <>
      {unassignedDevices.length > 0 && (
        <div
          className={`sidebar-section sidebar-stock${selectedDeviceInfo ? '' : ' full'}${dragOverStock ? ' drag-over' : ''}`}
          onDragOver={onStockDragOver}
          onDrop={onStockDrop}
        >
          <div className="sidebar-title">资源池</div>
          <div className="sidebar-content">
            {unassignedDevices.map(device => {
              const model = getDeviceModel(device.device_model_id);
              // 优先显示设备固有高度 height_u（下架后保留）；其次 U 位区间；最后型号高度
              const deviceU = device.height_u != null && device.height_u >= 1
                ? device.height_u
                : (device.start_u != null && device.end_u != null
                    ? device.end_u - device.start_u + 1
                    : (model?.height_u || 1));
              return (
                <div
                  key={device.id}
                  className={`lib-item ${draggingDeviceId === device.id ? 'dragging' : ''} ${selectedDeviceId === device.id ? 'selected' : ''}`}
                  draggable
                  onClick={() => onDeviceClick(device)}
                  onDoubleClick={() => onDeviceDoubleClick(device)}
                  onDragStart={(e) => onDragStart(e, device)}
                  onDragEnd={onDragEnd}
                >
                  <div className={`lib-icon ${safeDeviceType(model?.type)}`}>
                    {(model?.type || 'S').charAt(0).toUpperCase()}
                  </div>
                  <div className="lib-info">
                    <div className="lib-name">{device.name}</div>
                    <div className="lib-meta">{model?.name || '未知型号'} · {DEVICE_TYPE_LABELS[model?.type || ''] || model?.type || '-'} · {deviceU}U</div>
                  </div>
                  <span className="lib-tag">{deviceU}U</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {selectedDeviceInfo && (
        <DeviceDetailPanel
          selectedDeviceInfo={selectedDeviceInfo}
          onClose={onCloseDetail}
          onUpdate={onUpdateDetail}
          onRemove={onRemoveDetail}
        />
      )}
    </>
  );
}
