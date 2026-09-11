import type { Device, DeviceModel, Rack } from '../../types';
import { DEVICE_TYPE_LABELS, getStatusText, safeDeviceType } from '../../constants/labels';

export interface DropTarget {
  rackId: number;
  u: number;
  deviceHeight: number;
}

interface RackCardProps {
  rack: Rack;
  isFirst: boolean;
  isLast: boolean;
  selected: boolean;
  stats: { usedU: number; deviceCount: number };
  dropTarget: DropTarget | null;
  draggingDeviceId: number | null;
  selectedDeviceId: number | null;
  hoveredDeviceId: number | null;
  hasSearch: boolean;
  searchMatchedDeviceIds: Set<number>;
  devices: Device[];
  getDeviceModel: (id: number | null) => DeviceModel | undefined;
  getRoomName: (id: number | null) => string | undefined;
  onRackClick: (rack: Rack) => void;
  onRackDoubleClick: (rack: Rack) => void;
  onMoveLeft: (rack: Rack) => void;
  onMoveRight: (rack: Rack) => void;
  onDelete: (rackId: number) => void;
  onDragOver: (e: React.DragEvent, rackId: number, u: number) => void;
  onDragLeave: () => void;
  onDrop: (e: React.DragEvent, rackId: number, u: number) => void;
  onDeviceClick: (device: Device) => void;
  onDeviceDoubleClick: (device: Device) => void;
  onDeviceDragStart: (e: React.DragEvent, device: Device) => void;
  onDeviceDragEnd: () => void;
  onHoverDevice: (id: number | null) => void;
}

/** 单个机柜卡片：头部(移动/名称/删除) + U 位网格 + 设备块 + 使用率 */
export default function RackCard(props: RackCardProps) {
  const {
    rack, isFirst, isLast, selected, stats, dropTarget,
    draggingDeviceId, selectedDeviceId, hoveredDeviceId,
    hasSearch, searchMatchedDeviceIds, devices,
    getDeviceModel, getRoomName,
    onRackClick, onRackDoubleClick, onMoveLeft, onMoveRight, onDelete,
    onDragOver, onDragLeave, onDrop,
    onDeviceClick, onDeviceDoubleClick, onDeviceDragStart, onDeviceDragEnd, onHoverDevice,
  } = props;

  const uPct = rack.height_u > 0 ? (stats.usedU / rack.height_u) * 100 : 0;
  const uWarn = uPct > 85;

  return (
    <div
      className={`rack ${selected ? 'selected' : ''}`}
      onClick={() => onRackClick(rack)}
      onDoubleClick={() => onRackDoubleClick(rack)}
    >
      <div className="rack-header">
        <div className="rack-header-content">
          <div className="rack-move-btns">
            <button
              className="rack-move-btn"
              disabled={isFirst}
              onClick={(e) => { e.stopPropagation(); onMoveLeft(rack); }}
              title="左移"
            >◀</button>
            <button
              className="rack-move-btn"
              disabled={isLast}
              onClick={(e) => { e.stopPropagation(); onMoveRight(rack); }}
              title="右移"
            >▶</button>
          </div>
          <span className="rack-name-text">{rack.name}</span>
          <button
            className="rack-delete-btn"
            onClick={(e) => { e.stopPropagation(); onDelete(rack.id); }}
            title="删除机柜"
          >×</button>
        </div>
        <div className="rack-sub">
          <span>{rack.height_u}U</span>
          <span className="rack-sub-sep">·</span>
          <span>{stats.deviceCount}台设备</span>
          {rack.room_id && getRoomName(rack.room_id) && (
            <>
              <span className="rack-sub-sep">·</span>
              <span className="rack-location">{getRoomName(rack.room_id)}</span>
            </>
          )}
        </div>
      </div>

      <div className="rack-body" style={{ height: `${rack.height_u * 26}px` }}>
        <div className="rack-grid">
          {Array.from({ length: rack.height_u }, (_, i) => {
            const u = rack.height_u - i;
            const inDropRange = dropTarget?.rackId === rack.id &&
              u >= dropTarget.u && u < dropTarget.u + dropTarget.deviceHeight;
            return (
              <div
                key={`u-${u}`}
                className={`rack-u ${inDropRange ? 'drop-target' : ''}`}
                onDragOver={(e) => onDragOver(e, rack.id, u)}
                onDragLeave={onDragLeave}
                onDrop={(e) => onDrop(e, rack.id, u)}
              >
                <span className="u-number">{String(u).padStart(2, '0')}</span>
              </div>
            );
          })}
        </div>

        <div className="rack-devices">
          {devices.map(device => {
            const model = getDeviceModel(device.device_model_id);
            const startU = device.start_u ?? 0;
            const endU = device.end_u ?? 0;
            const deviceU = endU - startU + 1;
            const deviceHeight = deviceU * 26;
            const topPosition = (rack.height_u - endU) * 26;
            const isMatched = searchMatchedDeviceIds.has(device.id);

            return (
              <div
                key={`device-${device.id}`}
                className={[
                  'device-block',
                  `type-${safeDeviceType(model?.type)}`,
                  draggingDeviceId === device.id ? 'dragging' : '',
                  selectedDeviceId === device.id ? 'selected' : '',
                  deviceU === 1 ? 'device-1u' : '',
                  hasSearch && !isMatched ? 'search-dimmed' : '',
                ].filter(Boolean).join(' ')}
                onClick={(e) => { e.stopPropagation(); onDeviceClick(device); }}
                onDoubleClick={(e) => { e.stopPropagation(); onDeviceDoubleClick(device); }}
                onMouseEnter={() => onHoverDevice(device.id)}
                onMouseLeave={() => onHoverDevice(null)}
                draggable
                onDragStart={(e) => { e.stopPropagation(); onDeviceDragStart(e, device); }}
                onDragEnd={onDeviceDragEnd}
                style={{ top: `${topPosition}px`, height: `${deviceHeight}px` }}
              >
                <span className="dev-name">{device.name}</span>
                {deviceU > 1 && (
                  <span className="dev-model">{model?.name || 'Unknown'}</span>
                )}
                {device.status === 'online' && <span className="dev-status online pulse"></span>}
                {device.status === 'offline' && <span className="dev-status offline"></span>}
                {hoveredDeviceId === device.id && (
                  <div className="device-tooltip">
                    <div className="tooltip-row"><span>名称</span><span>{device.name}</span></div>
                    <div className="tooltip-row"><span>型号</span><span>{model?.name || '-'}</span></div>
                    <div className="tooltip-row"><span>类型</span><span>{model?.type ? DEVICE_TYPE_LABELS[model.type] || model.type : '-'}</span></div>
                    <div className="tooltip-row"><span>U位</span><span>{startU}-{endU}</span></div>
                    <div className="tooltip-row"><span>状态</span><span className={`tooltip-status ${device.status}`}>{getStatusText(device.status)}</span></div>
                  </div>
                )}
              </div>
            );
          })}
        </div>

        <div className="rack-footer">
          <div className="rack-meter" title={`U位使用: ${stats.usedU}U / ${rack.height_u}U`}>
            <div className="rack-meter-label">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="8" x2="22" y2="8"/><line x1="2" y1="16" x2="22" y2="16"/></svg>
              {stats.usedU}/{rack.height_u}U
            </div>
            <div className="rack-meter-bar">
              <div className={`rack-meter-fill u ${uWarn ? 'warn' : ''}`} style={{ width: `${uPct}%` }} />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
