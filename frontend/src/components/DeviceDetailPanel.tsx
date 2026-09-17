import { App } from 'antd';
import { Device, DeviceModel, Rack } from '../types';
import { DEVICE_TYPE_LABELS, getStatusText, safeDeviceType } from '../constants/labels';

interface DeviceDetailPanelProps {
  selectedDeviceInfo: {
    device: Device;
    model: DeviceModel | undefined;
    rack: Rack | undefined;
  } | null;
  onClose: () => void;
  onUpdate: (id: number, data: Partial<Device>) => Promise<void>;
  onRemove: (id: number) => void;
}

export default function DeviceDetailPanel({
  selectedDeviceInfo,
  onClose,
  onUpdate,
  onRemove,
}: DeviceDetailPanelProps) {
  const { modal } = App.useApp();

  if (!selectedDeviceInfo) return null;

  const handleRemoveDevice = () => {
    modal.confirm({
      title: '确认删除',
      content: `确定要删除设备「${selectedDeviceInfo.device.name}」吗？此操作不可撤销。`,
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        onRemove(selectedDeviceInfo.device.id);
        onClose();
      },
    });
  };

  return (
    <div className="sidebar-section sidebar-detail">
      <div className="sidebar-title">设备详情</div>
      <div className="sidebar-content">
        <div className="detail-header">
          <div className={`detail-type-badge ${safeDeviceType(selectedDeviceInfo.model?.type)}`}>
            {selectedDeviceInfo.model?.type ? DEVICE_TYPE_LABELS[selectedDeviceInfo.model.type] || '其他' : '未知'}
          </div>
          <div className={`detail-status-badge ${selectedDeviceInfo.device.status}`}>
            {getStatusText(selectedDeviceInfo.device.status)}
          </div>
        </div>
        <div className="detail-field">
          <span className="detail-label">名称</span>
          <span className="detail-value">{selectedDeviceInfo.device.name}</span>
        </div>
        <div className="detail-field">
          <span className="detail-label">型号</span>
          <span className="detail-value">{selectedDeviceInfo.model?.name || '-'}</span>
        </div>
        <div className="detail-field">
          <span className="detail-label">位置</span>
          <span className="detail-value">
            {selectedDeviceInfo.rack?.name || '未分配'} {selectedDeviceInfo.device.start_u != null ? `· ${selectedDeviceInfo.device.start_u}-${selectedDeviceInfo.device.end_u}U` : ''}
          </span>
        </div>
        <div className="detail-field">
          <span className="detail-label">IP</span>
          <span className="detail-value">{selectedDeviceInfo.device.ip_addresses || '-'}</span>
        </div>
        <div className="detail-field">
          <span className="detail-label">序列号</span>
          <span className="detail-value">{selectedDeviceInfo.device.serial_no || '-'}</span>
        </div>
        <div className="detail-field">
          <span className="detail-label">资产编号</span>
          <span className="detail-value detail-mono">{selectedDeviceInfo.device.asset_no || '-'}</span>
        </div>
        <div className="detail-field">
          <span className="detail-label">使用部门</span>
          <span className="detail-value">{selectedDeviceInfo.device.department || '-'}</span>
        </div>
        <div className="detail-field">
          <span className="detail-label">责任人</span>
          <span className="detail-value">{selectedDeviceInfo.device.owner || '-'}</span>
        </div>
        <div className="detail-field">
          <span className="detail-label">采购日期</span>
          <span className="detail-value">{selectedDeviceInfo.device.purchase_date || '-'}</span>
        </div>
        <div className="detail-status-toggle">
          <span className="detail-status-label">状态</span>
          <div className="status-toggle-btns">
            {selectedDeviceInfo.device.rack_id != null && (
              <>
                <button
                  className={`status-toggle-btn ${selectedDeviceInfo.device.status === 'online' ? 'active online' : ''}`}
                  onClick={async () => { await onUpdate(selectedDeviceInfo.device.id, { status: 'online' }); onClose(); }}
                >开机</button>
                <button
                  className={`status-toggle-btn ${selectedDeviceInfo.device.status === 'offline' ? 'active offline' : ''}`}
                  onClick={async () => { await onUpdate(selectedDeviceInfo.device.id, { status: 'offline' }); onClose(); }}
                >离线</button>
              </>
            )}
            <button
              className={`status-toggle-btn ${selectedDeviceInfo.device.status === 'unconfigured' ? 'active unconfigured' : ''}`}
              onClick={async () => {
                await onUpdate(selectedDeviceInfo.device.id, { status: 'unconfigured', rack_id: null, start_u: null, end_u: null });
                onClose();
              }}
            >未上架</button>
          </div>
        </div>
        <div className="detail-actions">
          <button className="detail-btn" onClick={onClose}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
            取消
          </button>
          <button className="detail-btn danger" onClick={handleRemoveDevice}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
            </svg>
            删除
          </button>
        </div>
      </div>
    </div>
  );
}
