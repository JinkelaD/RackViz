import { useState, useCallback, useMemo, useEffect } from 'react';
import { useOutletContext } from 'react-router-dom';
import { Modal, Form, Input, Button, Select } from 'antd';
import { useRacks } from '../hooks/useRacks';
import { useDevices } from '../hooks/useDevices';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { useRooms } from '../hooks/useRooms';
import { Rack, Device, DeviceModel, ViewMode, Room } from '../types';
import StatusBar, { RoomTabs } from '../components/StatusBar';

interface ContextType {
  view: ViewMode;
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  showAddRack: boolean;
  setShowAddRack: (v: boolean) => void;
  selectedRoomId: number | null;
  rooms: Room[];
  setSelectedRoomId: (id: number | null) => void;
  showRoomInput: boolean;
  setShowRoomInput: (v: boolean) => void;
  editingRoomId: number | null;
  setEditingRoomId: (id: number | null) => void;
  roomName: string;
  setRoomName: (name: string) => void;
  handleAddRoom: () => void;
  handleEditRoom: () => void;
  handleDeleteRoom: (id: number) => void;
  startEditRoom: (id: number, name: string) => void;
  updateRoom: (id: number, data: Partial<Room>) => Promise<void>;
}

const DEVICE_TYPE_LABELS: Record<string, string> = {
  server: '服务器',
  switch: '交换机',
  router: '路由器',
  storage: '存储',
  pdu: 'PDU',
  patch: '配线架',
};

function findAvailableSlot(
  targetU: number,
  deviceHeight: number,
  rackHeight: number,
  existingDevices: { start_u: number | null; end_u: number | null; id: number }[],
  excludeDeviceId?: number,
): { startU: number; endU: number } | null {
  const filtered = excludeDeviceId != null
    ? existingDevices.filter(d => d.id !== excludeDeviceId)
    : existingDevices;

  const occupied = new Set<number>();
  filtered.forEach(d => {
    if (d.start_u != null && d.end_u != null) {
      for (let i = d.start_u; i <= d.end_u; i++) occupied.add(i);
    }
  });

  const clamp = (s: number) => Math.max(1, Math.min(s, rackHeight));

  const candidates: number[] = [];

  let s = clamp(targetU);
  while (s >= 1) {
    const e = s + deviceHeight - 1;
    if (e > rackHeight) { s--; continue; }
    let blocked = false;
    for (let i = s; i <= e; i++) {
      if (occupied.has(i)) { blocked = true; break; }
    }
    if (!blocked) candidates.push(s);
    s--;
  }

  s = clamp(targetU) + 1;
  while (s + deviceHeight - 1 <= rackHeight) {
    const e = s + deviceHeight - 1;
    let blocked = false;
    for (let i = s; i <= e; i++) {
      if (occupied.has(i)) { blocked = true; break; }
    }
    if (!blocked) candidates.push(s);
    s++;
  }

  if (candidates.length === 0) return null;

  candidates.sort((a, b) => {
    const da = Math.abs(a - targetU);
    const db = Math.abs(b - targetU);
    if (da !== db) return da - db;
    return b - a;
  });

  const best = candidates[0];
  return { startU: best, endU: best + deviceHeight - 1 };
}

export default function RackView() {
  const ctx = useOutletContext<ContextType>();
  const { view, zoom, onZoomIn, onZoomOut, onZoomReset, searchQuery, onSearchChange, showAddRack, setShowAddRack, selectedRoomId } = ctx;
  const { racks, create: createRack, remove: removeRack, update: updateRack, updateQuiet: updateRackQuiet, refresh: refreshRacks } = useRacks();
  const { devices, update, remove } = useDevices();
  const { models } = useDeviceModels();
  const { rooms } = useRooms();

  const [selectedRackId, setSelectedRackId] = useState<number | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<Device | null>(null);
  const [draggingDevice, setDraggingDevice] = useState<Device | null>(null);
  const [dropTarget, setDropTarget] = useState<{ rackId: number; u: number } | null>(null);
  const [newRackName, setNewRackName] = useState('');
  const [newRackHeight, setNewRackHeight] = useState(42);
  const [newRackRoomId, setNewRackRoomId] = useState<number | null>(null);

  useEffect(() => {
    if (showAddRack) {
      setNewRackRoomId(selectedRoomId);
    }
  }, [showAddRack, selectedRoomId]);

  const [hoveredDeviceId, setHoveredDeviceId] = useState<number | null>(null);
  const [deviceDetailVisible, setDeviceDetailVisible] = useState(false);
  const [detailDevice, setDetailDevice] = useState<Device | null>(null);
  const [rackEditVisible, setRackEditVisible] = useState(false);
  const [editingRack, setEditingRack] = useState<Rack | null>(null);

  const filteredRacks = useMemo(() =>
    racks
      .filter(r => r.view === view)
      .filter(r => selectedRoomId === null || r.room_id === selectedRoomId)
      .sort((a, b) => a.sort_order - b.sort_order),
    [racks, view, selectedRoomId]);

  const getDevicesForRack = (rackId: number) => {
    return devices.filter(d => d.rack_id === rackId);
  };

  const getDeviceModel = (id: number | null) => models.find(m => m.id === id);
  const getRoomName = (id: number | null) => rooms.find(r => r.id === id)?.name;

  const unassignedDevices = useMemo(() => {
    return devices.filter(d => d.rack_id == null);
  }, [devices]);

  const rackStats = useMemo(() => {
    const stats: Record<number, { usedU: number; deviceCount: number }> = {};
    filteredRacks.forEach(rack => {
      const rackDevices = getDevicesForRack(rack.id);
      const usedU = rackDevices.reduce((s, d) => {
        if (d.start_u && d.end_u) return s + (d.end_u - d.start_u + 1);
        return s;
      }, 0);
      stats[rack.id] = { usedU, deviceCount: rackDevices.length };
    });
    return stats;
  }, [filteredRacks, devices]);

  const statusBarStats = useMemo(() => {
    if (selectedRackId) {
      const rackDevices = getDevicesForRack(selectedRackId);
      const rack = racks.find(r => r.id === selectedRackId);
      if (!rack) return null;
      const onlineCount = rackDevices.filter(d => d.status === 'online').length;
      const totalUUsed = rackDevices.reduce((s, d) => {
        if (d.start_u && d.end_u) return s + (d.end_u - d.start_u + 1);
        return s;
      }, 0);
      return {
        rackCount: 1,
        deviceCount: rackDevices.length,
        onlineCount,
        totalUUsed,
        totalU: rack.height_u,
        contextLabel: `机柜: ${rack.name}`,
      };
    }
    return null;
  }, [selectedRackId, racks, devices]);

  const viewStats = useMemo(() => {
    const viewDevices = filteredRacks.flatMap(r => getDevicesForRack(r.id));
    const onlineCount = viewDevices.filter(d => d.status === 'online').length;
    const totalUUsed = viewDevices.reduce((s, d) => {
      if (d.start_u && d.end_u) return s + (d.end_u - d.start_u + 1);
      return s;
    }, 0);
    const totalU = filteredRacks.reduce((sum, r) => sum + r.height_u, 0);
    return {
      rackCount: filteredRacks.length,
      deviceCount: viewDevices.length,
      onlineCount,
      totalUUsed,
      totalU,
    };
  }, [filteredRacks, devices]);

  const displayStats = statusBarStats || viewStats;

  const searchMatchedDeviceIds = useMemo(() => {
    if (!searchQuery.trim()) return new Set<number>();
    const q = searchQuery.toLowerCase();
    return new Set(
      devices.filter(d => d.name.toLowerCase().includes(q)).map(d => d.id)
    );
  }, [searchQuery, devices]);

  const handleRackClick = (rack: Rack) => {
    setSelectedRackId(rack.id);
    setSelectedDevice(null);
  };

  const handleDeviceClick = (device: Device) => {
    setSelectedDevice(device);
  };

  const handleDeviceDoubleClick = (device: Device) => {
    setDetailDevice(device);
    setDeviceDetailVisible(true);
  };

  const handleRackDoubleClick = (rack: Rack) => {
    setEditingRack(rack);
    setRackEditVisible(true);
  };

  const handleRackEditSave = async (values: { name: string; room_id: number | null }) => {
    if (!editingRack) return;
    await updateRack(editingRack.id, values);
    setRackEditVisible(false);
    setEditingRack(null);
  };

  const handleRemoveDevice = async () => {
    if (selectedDevice) {
      await remove(selectedDevice.id);
      setSelectedDevice(null);
    }
  };

  const handleUnassignDevice = async () => {
    if (selectedDevice) {
      await update(selectedDevice.id, {
        rack_id: null,
        start_u: null,
        end_u: null,
      });
      setSelectedDevice(null);
    }
  };

  const handleDragStart = useCallback((device: Device) => {
    setDraggingDevice(device);
  }, []);

  const handleDragEnd = useCallback(() => {
    setDraggingDevice(null);
    setDropTarget(null);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, rackId: number, u: number) => {
    e.preventDefault();
    setDropTarget({ rackId, u });
  }, []);

  const handleDragLeave = useCallback(() => {
    setDropTarget(null);
  }, []);

  const handleDrop = useCallback(async (e: React.DragEvent, rackId: number, u: number) => {
    e.preventDefault();

    const rack = racks.find(r => r.id === rackId);
    if (!rack || !draggingDevice) {
      handleDragEnd();
      return;
    }

    const deviceHeight = draggingDevice.end_u != null && draggingDevice.start_u != null
      ? draggingDevice.end_u - draggingDevice.start_u + 1
      : (getDeviceModel(draggingDevice.device_model_id)?.height_u || 1);

    const existingRackDevices = getDevicesForRack(rackId).filter(d => d.id !== draggingDevice.id)
      .map(d => ({ start_u: d.start_u, end_u: d.end_u, id: d.id }));

    const slot = findAvailableSlot(u, deviceHeight, rack.height_u, existingRackDevices);
    if (!slot) {
      handleDragEnd();
      return;
    }

    await update(draggingDevice.id, {
      rack_id: rackId,
      start_u: slot.startU,
      end_u: slot.endU,
    });

    handleDragEnd();
  }, [draggingDevice, racks, update, handleDragEnd, getDevicesForRack, getDeviceModel]);

  const handleAddRack = async () => {
    if (newRackName.trim()) {
      const maxOrder = racks
        .filter(r => r.view === view)
        .reduce((max, r) => Math.max(max, r.sort_order), -1);
      await createRack({
        name: newRackName.trim(),
        height_u: newRackHeight,
        row: 0,
        col: 0,
        view,
        sort_order: maxOrder + 1,
        room_id: newRackRoomId,
      });
      setNewRackName('');
      setNewRackHeight(42);
      setNewRackRoomId(null);
      setShowAddRack(false);
    }
  };

  const handleRemoveRack = async (rackId: number) => {
    await removeRack(rackId);
    if (selectedRackId === rackId) {
      setSelectedRackId(null);
    }
  };

  const handleMoveRackLeft = async (rack: Rack) => {
    const sameViewRacks = racks
      .filter(r => r.view === view)
      .sort((a, b) => a.sort_order - b.sort_order);
    const idx = sameViewRacks.findIndex(r => r.id === rack.id);
    if (idx <= 0) return;
    const prev = sameViewRacks[idx - 1];
    const aId = rack.id, aOrder = prev.sort_order;
    const bId = prev.id, bOrder = rack.sort_order;
    await updateRackQuiet(aId, { sort_order: aOrder });
    await updateRackQuiet(bId, { sort_order: bOrder });
    refreshRacks();
  };

  const handleMoveRackRight = async (rack: Rack) => {
    const sameViewRacks = racks
      .filter(r => r.view === view)
      .sort((a, b) => a.sort_order - b.sort_order);
    const idx = sameViewRacks.findIndex(r => r.id === rack.id);
    if (idx < 0 || idx >= sameViewRacks.length - 1) return;
    const next = sameViewRacks[idx + 1];
    const aId = rack.id, aOrder = next.sort_order;
    const bId = next.id, bOrder = rack.sort_order;
    await updateRackQuiet(aId, { sort_order: aOrder });
    await updateRackQuiet(bId, { sort_order: bOrder });
    refreshRacks();
  };

  const handleSidebarDeviceClick = (device: Device) => {
    setSelectedDevice(device);
    setSelectedRackId(null);
  };

  const selectedDeviceInfo = selectedDevice ? {
    ...selectedDevice,
    model: getDeviceModel(selectedDevice.device_model_id),
    rack: racks.find(r => r.id === selectedDevice.rack_id),
  } : null;

  const getStatusText = (status: string) => {
    if (status === 'online') return '在线';
    if (status === 'offline') return '离线';
    return '未配置';
  };

  return (
    <div className="rackview-wrapper">
      <div className="rackview-main">
      <div id="canvas-container">
        <div
          id="rack-grid"
          style={{
            transform: `scale(${zoom / 100})`,
          }}
        >
          {filteredRacks.length === 0 && (
            <div className="canvas-empty">
              <div className="canvas-empty-icon">⊞</div>
              <div className="canvas-empty-title">
                {view === 'front' ? '暂无正面机柜' : '暂无背面机柜'}
              </div>
              <div className="canvas-empty-hint">点击底部「+ 机柜」添加新机柜</div>
            </div>
          )}
          {filteredRacks.map((rack, rackIdx) => {
            const stats = rackStats[rack.id] || { usedU: 0, deviceCount: 0 };
            const uPct = rack.height_u > 0 ? (stats.usedU / rack.height_u) * 100 : 0;
            const uWarn = uPct > 85;
            const isFirst = rackIdx === 0;
            const isLast = rackIdx === filteredRacks.length - 1;

            return (
            <div
              key={rack.id}
              className={`rack ${selectedRackId === rack.id ? 'selected' : ''}`}
              onClick={() => handleRackClick(rack)}
              onDoubleClick={() => handleRackDoubleClick(rack)}
            >
              <div className="rack-header">
                <div className="rack-header-content">
                  <div className="rack-move-btns">
                    <button
                      className="rack-move-btn"
                      disabled={isFirst}
                      onClick={(e) => { e.stopPropagation(); handleMoveRackLeft(rack); }}
                      title="左移"
                    >◀</button>
                    <button
                      className="rack-move-btn"
                      disabled={isLast}
                      onClick={(e) => { e.stopPropagation(); handleMoveRackRight(rack); }}
                      title="右移"
                    >▶</button>
                  </div>
                  <span className="rack-name-text">{rack.name}</span>
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
                    const deviceU = device.end_u! - device.start_u! + 1;
                    const isMatched = searchMatchedDeviceIds.has(device.id);
                    const hasSearch = searchQuery.trim().length > 0;
                    return (
                      <div
                        key={`device-${device.id}`}
                        className={[
                          'device-block',
                          `type-${model?.type || 'server'}`,
                          draggingDevice?.id === device.id ? 'dragging' : '',
                          selectedDevice?.id === device.id ? 'selected' : '',
                          deviceU === 1 ? 'device-1u' : '',
                          hasSearch && !isMatched ? 'search-dimmed' : '',
                        ].filter(Boolean).join(' ')}
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeviceClick(device);
                        }}
                        onDoubleClick={(e) => {
                          e.stopPropagation();
                          handleDeviceDoubleClick(device);
                        }}
                        onMouseEnter={() => setHoveredDeviceId(device.id)}
                        onMouseLeave={() => setHoveredDeviceId(null)}
                        draggable
                        onDragStart={(e) => {
                          e.stopPropagation();
                          handleDragStart(device);
                        }}
                        onDragEnd={handleDragEnd}
                        style={{
                          top: `${topPosition}px`,
                          height: `${deviceHeight}px`,
                        }}
                      >
                        <span className="dev-name">{device.name}</span>
                        {deviceU > 1 && (
                          <span className="dev-model">{model?.name || 'Unknown'}</span>
                        )}
                        {device.status === 'online' && (
                          <span className="dev-status online pulse"></span>
                        )}
                        {device.status === 'offline' && (
                          <span className="dev-status offline"></span>
                        )}
                        {hoveredDeviceId === device.id && (
                          <div className="device-tooltip">
                            <div className="tooltip-row"><span>名称</span><span>{device.name}</span></div>
                            <div className="tooltip-row"><span>型号</span><span>{model?.name || '-'}</span></div>
                            <div className="tooltip-row"><span>类型</span><span>{model?.type ? DEVICE_TYPE_LABELS[model.type] || model.type : '-'}</span></div>
                            <div className="tooltip-row"><span>U位</span><span>{device.start_u}-{device.end_u}</span></div>
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
          })}
        </div>
      </div>

      <aside id="sidebar">
        {unassignedDevices.length > 0 && (
        <div className={`sidebar-section sidebar-stock${selectedDeviceInfo ? '' : ' full'}`}>
          <div className="sidebar-title">库存设备</div>
          <div className="sidebar-content">
            {unassignedDevices.map(device => {
              const model = getDeviceModel(device.device_model_id);
              const deviceU = device.start_u != null && device.end_u != null
                ? device.end_u - device.start_u + 1
                : (model?.height_u || 1);
              return (
                <div
                  key={device.id}
                  className={`lib-item ${draggingDevice?.id === device.id ? 'dragging' : ''} ${selectedDevice?.id === device.id ? 'selected' : ''}`}
                  draggable
                  onClick={() => handleSidebarDeviceClick(device)}
                  onDoubleClick={() => handleDeviceDoubleClick(device)}
                  onDragStart={(e) => {
                    handleDragStart(device);
                  }}
                  onDragEnd={handleDragEnd}
                >
                  <div className={`lib-icon ${model?.type || 'server'}`}>
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
        <div className="sidebar-section sidebar-detail">
          <div className="sidebar-title">设备详情</div>
          <div className="sidebar-content">
                <div className="detail-header">
                  <div className={`detail-type-badge ${selectedDeviceInfo.model?.type || 'server'}`}>
                    {selectedDeviceInfo.model?.type ? DEVICE_TYPE_LABELS[selectedDeviceInfo.model.type] || selectedDeviceInfo.model.type : '未知'}
                  </div>
                  <div className={`detail-status-badge ${selectedDeviceInfo.status}`}>
                    {getStatusText(selectedDeviceInfo.status)}
                  </div>
                </div>
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
                    {selectedDeviceInfo.rack?.name || '未分配'} {selectedDeviceInfo.start_u != null ? `· ${selectedDeviceInfo.start_u}-${selectedDeviceInfo.end_u}U` : ''}
                  </span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">IP</span>
                  <span className="detail-value">{selectedDeviceInfo.ip_addresses || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">序列号</span>
                  <span className="detail-value">{selectedDeviceInfo.serial_no || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">资产编号</span>
                  <span className="detail-value" style={{ fontFamily: 'var(--font-mono)', color: 'var(--accent-cyan)' }}>{selectedDeviceInfo.asset_no || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">使用部门</span>
                  <span className="detail-value">{selectedDeviceInfo.department || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">责任人</span>
                  <span className="detail-value">{selectedDeviceInfo.owner || '-'}</span>
                </div>
                <div className="detail-field">
                  <span className="detail-label">采购日期</span>
                  <span className="detail-value">{selectedDeviceInfo.purchase_date || '-'}</span>
                </div>
                <div className="detail-actions">
                  <button className="detail-btn" onClick={() => setSelectedDevice(null)}>取消选择</button>
                  {selectedDeviceInfo.rack_id != null && (
                    <button className="detail-btn" onClick={handleUnassignDevice}>移出机柜</button>
                  )}
                  <button className="detail-btn danger" onClick={handleRemoveDevice}>删除设备</button>
                </div>
          </div>
        </div>
        )}
      </aside>

      </div>

      <RoomTabs
        rooms={ctx.rooms}
        selectedRoomId={ctx.selectedRoomId}
        setSelectedRoomId={ctx.setSelectedRoomId}
        showRoomInput={ctx.showRoomInput}
        setShowRoomInput={ctx.setShowRoomInput}
        editingRoomId={ctx.editingRoomId}
        setEditingRoomId={ctx.setEditingRoomId}
        roomName={ctx.roomName}
        setRoomName={ctx.setRoomName}
        handleAddRoom={ctx.handleAddRoom}
        handleEditRoom={ctx.handleEditRoom}
        handleDeleteRoom={ctx.handleDeleteRoom}
        startEditRoom={ctx.startEditRoom}
        updateRoom={ctx.updateRoom}
      />

      <StatusBar
        zoom={zoom}
        onZoomIn={onZoomIn}
        onZoomOut={onZoomOut}
        onZoomReset={onZoomReset}
        searchQuery={searchQuery}
        onSearchChange={onSearchChange}
        onAddRack={() => setShowAddRack(true)}
        deviceCount={displayStats.deviceCount}
        onlineCount={displayStats.onlineCount}
        totalUUsed={displayStats.totalUUsed}
        totalU={displayStats.totalU}
      />

      {showAddRack && (
        <div className="modal-overlay" onClick={() => setShowAddRack(false)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              添加机柜
              <button className="modal-close" onClick={() => setShowAddRack(false)}>×</button>
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
              <div className="form-group">
                <label className="form-label">机柜高度（U）</label>
                <input
                  className="form-input"
                  type="number"
                  min={4}
                  max={48}
                  value={newRackHeight}
                  onChange={e => setNewRackHeight(Number(e.target.value))}
                />
              </div>
              <div className="form-group">
                <label className="form-label">所属机房</label>
                <select
                  className="form-input"
                  value={newRackRoomId || ''}
                  onChange={e => setNewRackRoomId(e.target.value ? Number(e.target.value) : null)}
                >
                  <option value="">选择机房</option>
                  {rooms.map(room => (
                    <option key={room.id} value={room.id}>{room.name}</option>
                  ))}
                </select>
              </div>
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={() => setShowAddRack(false)}>取消</button>
              <button className="btn primary" onClick={handleAddRack}>确定</button>
            </div>
          </div>
        </div>
      )}

      <Modal
        title={null}
        open={deviceDetailVisible}
        onCancel={() => { setDeviceDetailVisible(false); setDetailDevice(null); }}
        footer={null}
        width={440}
        destroyOnHidden
        className="device-edit-modal"
      >
        {detailDevice && (() => {
          const dm = getDeviceModel(detailDevice.device_model_id);
          const dr = racks.find(r => r.id === detailDevice.rack_id);
          return (
            <div className="device-edit-wrap">
              <div className="device-edit-header">
                <div className="device-edit-icon">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" strokeWidth="1.5">
                    <rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="6" x2="22" y2="6"/><line x1="2" y1="10" x2="22" y2="10"/><line x1="2" y1="14" x2="22" y2="14"/><line x1="2" y1="18" x2="22" y2="18"/>
                  </svg>
                </div>
                <div className="device-edit-title">编辑设备</div>
                <div className="device-edit-subtitle">
                  {dr?.name || '未分配'}
                  {detailDevice.start_u != null ? ` · U${detailDevice.start_u}-${detailDevice.end_u}` : ''}
                </div>
              </div>
              <Form
                layout="vertical"
                size="small"
                initialValues={{
                  name: detailDevice.name,
                  device_model_id: detailDevice.device_model_id,
                  serial_no: detailDevice.serial_no || '',
                  asset_no: detailDevice.asset_no || '',
                  ip_addresses: detailDevice.ip_addresses || '',
                  department: detailDevice.department || '',
                  owner: detailDevice.owner || '',
                  purchase_date: detailDevice.purchase_date || '',
                }}
                onFinish={async (values) => {
                  await update(detailDevice.id, values);
                  setDeviceDetailVisible(false);
                  setDetailDevice(null);
                }}
              >
                <div className="device-edit-body">
                  <Form.Item name="name" label="设备名称" rules={[{ required: true }]}>
                    <Input />
                  </Form.Item>
                  <Form.Item name="device_model_id" label="设备型号">
                    <Select placeholder="选择型号" allowClear>
                      {models.map(m => (
                        <Select.Option key={m.id} value={m.id}>{m.name} ({m.height_u}U)</Select.Option>
                      ))}
                    </Select>
                  </Form.Item>
                  <Form.Item name="ip_addresses" label="IP地址">
                    <Input placeholder="多个IP用逗号分隔" />
                  </Form.Item>
                  <Form.Item name="serial_no" label="序列号">
                    <Input />
                  </Form.Item>
                  <Form.Item name="asset_no" label="资产编号">
                    <Input />
                  </Form.Item>
                  <Form.Item name="department" label="使用部门">
                    <Input />
                  </Form.Item>
                  <Form.Item name="owner" label="责任人">
                    <Input />
                  </Form.Item>
                  <Form.Item name="purchase_date" label="采购日期">
                    <Input type="date" />
                  </Form.Item>
                </div>
                <div className="device-edit-footer">
                  <Button onClick={() => { setDeviceDetailVisible(false); setDetailDevice(null); }}>
                    取消
                  </Button>
                  <Button type="primary" htmlType="submit">
                    保存修改
                  </Button>
                </div>
              </Form>
            </div>
          );
        })()}
      </Modal>

      <Modal
        title={null}
        open={rackEditVisible}
        onCancel={() => { setRackEditVisible(false); setEditingRack(null); }}
        footer={null}
        width={420}
        destroyOnHidden
        className="device-edit-modal"
      >
        {editingRack && (() => {
          return (
          <div className="device-edit-wrap">
            <div className="device-edit-header">
              <div className="device-edit-icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" strokeWidth="1.5">
                  <rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="8" x2="22" y2="8"/><line x1="2" y1="16" x2="22" y2="16"/><line x1="8" y1="2" x2="8" y2="22"/>
                </svg>
              </div>
              <div className="device-edit-title">编辑机柜</div>
              <div className="device-edit-subtitle">{editingRack.name} · {editingRack.height_u}U</div>
            </div>
            <Form
              layout="vertical"
              size="small"
              initialValues={{
                name: editingRack.name,
                room_id: editingRack.room_id || undefined,
              }}
              onFinish={handleRackEditSave}
            >
              <div className="device-edit-body">
                <Form.Item name="name" label="机柜名称" rules={[{ required: true }]}>
                  <Input />
                </Form.Item>
                <Form.Item name="room_id" label="所属机房">
                  <Select placeholder="选择机房" allowClear>
                    {rooms.map(r => (
                      <Select.Option key={r.id} value={r.id}>{r.name}</Select.Option>
                    ))}
                  </Select>
                </Form.Item>
              </div>
              <div className="device-edit-footer">
                <Button onClick={() => { setRackEditVisible(false); setEditingRack(null); }}>
                  取消
                </Button>
                <Button type="primary" htmlType="submit">
                  保存修改
                </Button>
              </div>
            </Form>
          </div>
        );
        })()}
      </Modal>

    </div>
  );
}
