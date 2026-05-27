import { useState, useCallback } from 'react';
import { useOutletContext } from 'react-router-dom';
import { useRacks } from '../hooks/useRacks';
import { useDevices } from '../hooks/useDevices';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { Rack, Device, DeviceModel, EditMode, ViewMode } from '../types';

interface ContextType {
  mode: EditMode;
  view: ViewMode;
  zoom: number;
  searchQuery: string;
}

export default function RackView() {
  const { mode, view, zoom, searchQuery } = useOutletContext<ContextType>();
  const { racks, create: createRack, remove: removeRack } = useRacks();
  const { devices, create, update, remove } = useDevices();
  const { models } = useDeviceModels();
  const [selectedRackId, setSelectedRackId] = useState<number | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<Device | null>(null);
  const [draggingModel, setDraggingModel] = useState<DeviceModel | null>(null);
  const [draggingDevice, setDraggingDevice] = useState<Device | null>(null);
  const [dropTarget, setDropTarget] = useState<{ rackId: number; u: number } | null>(null);
  const [showAddRackModal, setShowAddRackModal] = useState(false);
  const [newRackName, setNewRackName] = useState('');

  const filteredRacks = racks.filter(r => r.view === view);
  
  const getDevicesForRack = (rackId: number) => {
    return devices.filter(d => d.rack_id === rackId);
  };

  const getDeviceModel = (modelId: number | null): DeviceModel | undefined => {
    return models.find(m => m.id === modelId);
  };

  const getDeviceTypeColor = (type: string) => {
    const colors: Record<string, string> = {
      server: '#5A8FD4',
      switch: '#3DC9B0',
      router: '#D4A85A',
      storage: '#8B5AD4',
      pdu: '#D45A5A',
      patch: '#5C6170',
    };
    return colors[type] || '#5A8FD4';
  };

  const handleRackClick = (rack: Rack) => {
    setSelectedRackId(rack.id);
    setSelectedDevice(null);
  };

  const handleDeviceClick = (device: Device) => {
    setSelectedDevice(device);
  };

  const handleRemoveDevice = async () => {
    if (selectedDevice) {
      await remove(selectedDevice.id);
      setSelectedDevice(null);
    }
  };

  const handleUpdateDevice = async (updates: Partial<Device>) => {
    if (selectedDevice) {
      await update(selectedDevice.id, updates);
    }
  };

  const handleDragStart = useCallback((data: { type: 'model'; model: DeviceModel } | { type: 'device'; device: Device }) => {
    if (data.type === 'model') {
      setDraggingModel(data.model);
    } else {
      setDraggingDevice(data.device);
    }
  }, []);

  const handleDragEnd = useCallback(() => {
    setDraggingModel(null);
    setDraggingDevice(null);
    setDropTarget(null);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, rackId: number, u: number) => {
    e.preventDefault();
    if (mode === 'drag') {
      setDropTarget({ rackId, u });
    }
  }, [mode]);

  const handleDragLeave = useCallback(() => {
    setDropTarget(null);
  }, []);

  const handleDrop = useCallback(async (e: React.DragEvent, rackId: number, u: number) => {
    e.preventDefault();

    const rack = racks.find(r => r.id === rackId);
    if (!rack) return;

    if (draggingDevice) {
      const deviceHeight = draggingDevice.end_u! - draggingDevice.start_u! + 1;
      let startU = u;
      let endU = u + deviceHeight - 1;
      
      if (endU > rack.height_u) {
        startU = rack.height_u - deviceHeight + 1;
        endU = rack.height_u;
      }

      if (startU < 1) {
        handleDragEnd();
        return;
      }

      const rackDevices = getDevicesForRack(rackId).filter(d => d.id !== draggingDevice.id);
      const isOccupied = rackDevices.some(d => {
        if (d.start_u === null || d.end_u === null) return false;
        return !(d.end_u < startU || d.start_u > endU);
      });

      if (!isOccupied) {
        await update(draggingDevice.id, {
          rack_id: rackId,
          start_u: startU,
          end_u: endU,
        });
      }
    } else if (draggingModel) {
      const deviceHeight = draggingModel.height_u;
      let startU = u;
      let endU = u + deviceHeight - 1;
      
      if (endU > rack.height_u) {
        startU = rack.height_u - deviceHeight + 1;
        endU = rack.height_u;
      }

      if (startU < 1) {
        handleDragEnd();
        return;
      }

      const rackDevices = getDevicesForRack(rackId);
      const isOccupied = rackDevices.some(d => {
        if (d.start_u === null || d.end_u === null) return false;
        return !(d.end_u < startU || d.start_u > endU);
      });

      if (!isOccupied) {
        await create({
          name: `${draggingModel.name} (新设备)`,
          device_model_id: draggingModel.id,
          rack_id: rackId,
          start_u: startU,
          end_u: endU,
          power_watt: draggingModel.power_watt,
          status: 'unconfigured',
        });
      }
    }

    handleDragEnd();
  }, [draggingDevice, draggingModel, racks, create, update, getDevicesForRack, handleDragEnd]);

  const filteredDevices = searchQuery
    ? devices.filter(d => d.name.toLowerCase().includes(searchQuery.toLowerCase()))
    : [];

  const handleAddRack = async () => {
    if (newRackName.trim()) {
      await createRack({
        name: newRackName.trim(),
        height_u: 42,
        row: racks.length + 1,
        col: 1,
        view: 'front',
      });
      setNewRackName('');
      setShowAddRackModal(false);
    }
  };

  const handleRemoveRack = async (rackId: number) => {
    await removeRack(rackId);
    if (selectedRackId === rackId) {
      setSelectedRackId(null);
    }
  };

  const selectedDeviceInfo = selectedDevice ? {
    ...selectedDevice,
    model: getDeviceModel(selectedDevice.device_model_id),
    rack: racks.find(r => r.id === selectedDevice.rack_id),
  } : null;

  return (
    <>
      <div id="canvas-container">
        <div 
          id="rack-grid" 
          style={{ 
            gridTemplateColumns: `repeat(${Math.max(filteredRacks.length, 1)}, var(--rack-min-w))`,
            transform: `scale(${zoom / 100})`,
          }}
        >
          {filteredRacks.map(rack => (
            <div
              key={rack.id}
              className={`rack ${selectedRackId === rack.id ? 'selected' : ''}`}
              onClick={() => handleRackClick(rack)}
            >
              <div className="rack-header">
                <div className="rack-header-content">
                  {rack.name}
                  <button 
                    className="rack-delete-btn" 
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemoveRack(rack.id);
                    }}
                    title="删除机柜"
                  >
                    ×
                  </button>
                </div>
                <div className="rack-sub">{rack.height_u}U · Row{rack.row} Col{rack.col}</div>
              </div>
              <div className="rack-body">
                <div className="rack-grid">
                  {Array.from({ length: rack.height_u }, (_, i) => {
                    const u = rack.height_u - i;
                    return (
                      <div 
                        key={`u-${u}`} 
                        className={`rack-u ${dropTarget?.rackId === rack.id && dropTarget?.u === u ? 'drop-target' : ''}`}
                        onDragOver={(e) => handleDragOver(e, rack.id, u)}
                        onDragLeave={handleDragLeave}
                        onDrop={(e) => handleDrop(e, rack.id, u)}
                      >
                        <span className="u-number">{String(u).padStart(2, '0')}</span>
                      </div>
                    );
                  })}
                </div>
                <div className="rack-devices">
                  {getDevicesForRack(rack.id).map(device => {
                    const model = getDeviceModel(device.device_model_id);
                    const deviceHeight = (device.end_u! - device.start_u! + 1) * 26;
                    const topPosition = (rack.height_u - device.end_u!) * 26;
                    return (
                      <div
                        key={`device-${device.id}`}
                        className={`device-block type-${model?.type || 'server'} ${draggingDevice?.id === device.id ? 'dragging' : ''}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeviceClick(device);
                        }}
                        draggable
                        onDragStart={(e) => {
                          e.stopPropagation();
                          handleDragStart({ type: 'device', device });
                        }}
                        onDragEnd={handleDragEnd}
                        style={{ 
                          top: `${topPosition}px`,
                          height: `${deviceHeight}px`,
                        }}
                      >
                        <span className="dev-name">{device.name}</span>
                        <span className="dev-model">{model?.name || 'Unknown'}</span>
                        {device.status === 'online' && (
                          <span className="dev-status online pulse"></span>
                        )}
                        {device.status === 'offline' && (
                          <span className="dev-status offline"></span>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>

      <aside id="sidebar">
        <div className="sidebar-section">
          <div className="sidebar-title">设备库 · 拖放到机柜</div>
          <div className="sidebar-content">
            {models.map(model => (
              <div
                key={model.id}
                className={`lib-item ${draggingModel?.id === model.id ? 'dragging' : ''}`}
                draggable
                onDragStart={() => handleDragStart({ type: 'model', model })}
                onDragEnd={handleDragEnd}
              >
                <div className={`lib-icon ${model.type}`}>
                  {model.type.charAt(0).toUpperCase()}
                </div>
                <div className="lib-info">
                  <div className="lib-name">{model.name}</div>
                  <div className="lib-meta">{model.type} · {model.height_u}U · {model.power_watt}W</div>
                </div>
                <span className="lib-tag">{model.height_u}U</span>
              </div>
            ))}
          </div>
        </div>

        <div className="sidebar-section">
          <div className="sidebar-title">设备详情</div>
          <div className="sidebar-content">
            {selectedDeviceInfo ? (
              <>
                <div className="detail-field">
                  <span className="detail-label">名称</span>
                  <span className="detail-value">{selectedDeviceInfo.name}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">型号</span>
                  <span className="detail-value">{selectedDeviceInfo.model?.name || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">位置</span>
                  <span className="detail-value">
                    {selectedDeviceInfo.rack?.name || '-'} · {selectedDeviceInfo.start_u}-{selectedDeviceInfo.end_u}U
                  </span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">IP</span>
                  <span className="detail-value">{selectedDeviceInfo.ip_addresses || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">状态</span>
                  <span className={`detail-status ${selectedDeviceInfo.status}`}>
                    {selectedDeviceInfo.status === 'online' ? '在线' : 
                     selectedDeviceInfo.status === 'offline' ? '离线' : '未配置'}
                  </span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">功率</span>
                  <span className="detail-value">{selectedDeviceInfo.power_watt}W</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">序列号</span>
                  <span className="detail-value">{selectedDeviceInfo.serial_no || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">功能</span>
                  <span className="detail-value">{selectedDeviceInfo.function || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">采购日期</span>
                  <span className="detail-value">{selectedDeviceInfo.purchase_date || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">质保到期</span>
                  <span className="detail-value">{selectedDeviceInfo.warranty_expire || '-'}</span>
                </div>
                <div className="detail-actions">
                  <button className="detail-btn" onClick={() => setSelectedDevice(null)}>取消选择</button>
                  <button className="detail-btn danger" onClick={handleRemoveDevice}>删除设备</button>
                </div>
              </>
            ) : (
              <div className="empty-state">
                <div className="empty-icon">◉</div>
                <div className="empty-text">选择设备查看详情</div>
                <div className="empty-hint">点击机柜中的设备或从设备库拖放新设备</div>
              </div>
            )}
          </div>
        </div>
      </aside>

      {showAddRackModal && (
        <div className="modal-overlay" onClick={() => setShowAddRackModal(false)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              添加机柜
              <button className="modal-close" onClick={() => setShowAddRackModal(false)}>×</button>
            </div>
            <div className="modal-body">
              <div className="form-group">
                <label className="form-label">机柜名称</label>
                <input 
                  className="form-input" 
                  type="text" 
                  value={newRackName}
                  onChange={e => setNewRackName(e.target.value)}
                  placeholder="输入机柜名称"
                  autoFocus
                />
              </div>
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setShowAddRackModal(false)}>取消</button>
              <button className="btn primary" onClick={handleAddRack}>确定</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}