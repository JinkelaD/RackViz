import { useState, useCallback, useMemo } from 'react';
import { App } from 'antd';
import { useRacks } from './useRacks';
import { useDevices } from './useDevices';
import { useDeviceModels } from './useDeviceModels';
import { useViewContext } from '../contexts/ViewContext';
import { useRoomContext } from '../contexts/RoomContext';
import { useUndo } from '../contexts/UndoContext';
import { useRegisterShortcuts } from './useGlobalShortcuts';
import { Device, DeviceModel, Room, Rack } from '../types';
import { findAvailableSlot } from '../utils/rackLayout';
import * as tauriApi from '../tauri-api';
import type { SelectedDeviceInfo } from '../components/rack/DeviceStockPanel';

export interface RackViewStats {
  rackCount: number;
  deviceCount: number;
  onlineCount: number;
  totalUUsed: number;
  totalU: number;
  contextLabel?: string;
}

/**
 * RackView 页状态与交互逻辑（从页面抽离，页面只保留 JSX 装配）。
 * 数据钩子 + 视图/房间上下文 + 拖拽/选中/弹窗状态全部收敛于此。
 */
export function useRackView() {
  const { modal, message } = App.useApp();
  const { pushUndo, undo } = useUndo();

  const viewCtx = useViewContext();
  const {
    view, zoom, onZoomIn, onZoomOut, onZoomReset,
    searchQuery, onSearchChange, showAddRack, setShowAddRack,
  } = viewCtx;
  const { rooms, selectedRoomId } = useRoomContext();
  const {
    racks, create: createRack, remove: removeRack,
    update: updateRack, updateQuiet: updateRackQuiet, refresh: refreshRacks,
  } = useRacks();
  const { devices, update, remove, refresh: refreshDevices } = useDevices();
  const { models } = useDeviceModels();

  const [selectedRackId, setSelectedRackId] = useState<number | null>(null);
  const [selectedDevice, setSelectedDevice] = useState<Device | null>(null);
  const [draggingDevice, setDraggingDevice] = useState<Device | null>(null);
  const [dropTarget, setDropTarget] = useState<{ rackId: number; u: number; deviceHeight: number; deviceName: string } | null>(null);
  const [dragOverStock, setDragOverStock] = useState(false);

  const [hoveredDeviceId, setHoveredDeviceId] = useState<number | null>(null);
  const [deviceDetailVisible, setDeviceDetailVisible] = useState(false);
  const [detailDevice, setDetailDevice] = useState<Device | null>(null);
  const [rackEditVisible, setRackEditVisible] = useState(false);
  const [editingRack, setEditingRack] = useState<Rack | null>(null);

  // ---------- 派生数据（Map 稳定引用，避免重复 filter/find） ----------

  const filteredRacks = useMemo(() =>
    racks
      .filter(r => r.view === view)
      .filter(r => selectedRoomId === null || r.room_id === selectedRoomId)
      .sort((a, b) => a.sort_order - b.sort_order),
    [racks, view, selectedRoomId]);

  const devicesByRack = useMemo(() => {
    const map = new Map<number, Device[]>();
    devices.forEach(d => {
      if (d.rack_id == null) return;
      const list = map.get(d.rack_id);
      if (list) list.push(d);
      else map.set(d.rack_id, [d]);
    });
    return map;
  }, [devices]);

  const getDevicesForRack = useCallback((rackId: number): Device[] => {
    return devicesByRack.get(rackId) ?? [];
  }, [devicesByRack]);

  const modelsById = useMemo(() => {
    const map = new Map<number, DeviceModel>();
    models.forEach(m => map.set(m.id, m));
    return map;
  }, [models]);

  const roomsById = useMemo(() => {
    const map = new Map<number, Room>();
    rooms.forEach(r => map.set(r.id, r));
    return map;
  }, [rooms]);

  const getDeviceModel = useCallback((id: number | null): DeviceModel | undefined => {
    return id == null ? undefined : modelsById.get(id);
  }, [modelsById]);

  const getRoomName = useCallback((id: number | null): string | undefined => {
    return id == null ? undefined : roomsById.get(id)?.name;
  }, [roomsById]);

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
  }, [filteredRacks, getDevicesForRack]);

  const statusBarStats = useMemo((): RackViewStats | null => {
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
  }, [selectedRackId, racks, getDevicesForRack]);

  const viewStats = useMemo((): RackViewStats => {
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
  }, [filteredRacks, getDevicesForRack]);

  const displayStats = statusBarStats || viewStats;

  const searchMatchedDeviceIds = useMemo(() => {
    if (!searchQuery.trim()) return new Set<number>();
    const q = searchQuery.toLowerCase();
    return new Set(
      devices.filter(d => d.name.toLowerCase().includes(q)).map(d => d.id)
    );
  }, [searchQuery, devices]);

  // ---------- 交互 handlers ----------

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

  const handleAddRackSubmit = useCallback(async (name: string, height: number, roomId: number | null) => {
    const maxOrder = racks
      .filter(r => r.view === view)
      .reduce((max, r) => Math.max(max, r.sort_order), -1);
    await createRack({
      name,
      height_u: height,
      row: 0,
      col: 0,
      view,
      sort_order: maxOrder + 1,
      room_id: roomId,
    });
    setShowAddRack(false);
  }, [racks, view, createRack, setShowAddRack]);

  const handleDetailDeviceSave = useCallback(async (id: number, values: Record<string, unknown>) => {
    const prev = devices.find(d => d.id === id);
    await update(id, values as Partial<Device>);
    if (prev) {
      const inverse: Partial<Device> = {};
      Object.keys(values).forEach(k => {
        (inverse as Record<string, unknown>)[k] = (prev as unknown as Record<string, unknown>)[k];
      });
      pushUndo({ label: `编辑设备「${prev.name}」`, undo: async () => { await update(id, inverse); } });
    }
    setDeviceDetailVisible(false);
    setDetailDevice(null);
  }, [devices, update, pushUndo]);

  const handleRackEditSave = useCallback(async (id: number, values: { name: string; room_id: number | null }) => {
    await updateRack(id, values);
    setRackEditVisible(false);
    setEditingRack(null);
  }, [updateRack]);

  const handleDragStart = useCallback((e: React.DragEvent, device: Device) => {
    e.dataTransfer.setData('text/plain', String(device.id));
    e.dataTransfer.effectAllowed = 'move';
    setDraggingDevice(device);
  }, []);

  const handleDragEnd = useCallback(() => {
    setDraggingDevice(null);
    setDropTarget(null);
    setDragOverStock(false);
  }, []);

  /**
   * 计算设备上架时的占用高度（U）。
   * 优先级：设备当前 U 位区间（在架移动）> 设备固有高度 height_u（下架后保留）> 型号高度 > 1。
   * 修复回归：多U设备下架（start/end 清空）后重上架退化为 1U。
   */
  const resolveDeviceHeight = useCallback((device: Device, model?: DeviceModel): number => {
    if (device.start_u != null && device.end_u != null && device.end_u >= device.start_u) {
      return device.end_u - device.start_u + 1;
    }
    if (device.height_u != null && device.height_u >= 1) {
      return device.height_u;
    }
    return model?.height_u || 1;
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, rackId: number, u: number) => {
    e.preventDefault();
    const model = draggingDevice ? getDeviceModel(draggingDevice.device_model_id) : undefined;
    const deviceHeight = draggingDevice ? resolveDeviceHeight(draggingDevice, model) : 1;
    setDropTarget({ rackId, u, deviceHeight, deviceName: draggingDevice?.name ?? '' });
  }, [draggingDevice, getDeviceModel, resolveDeviceHeight]);

  const handleDragLeave = useCallback(() => {
    setDropTarget(null);
  }, []);

  const handleDrop = useCallback(async (e: React.DragEvent, rackId: number, u: number) => {
    e.preventDefault();
    e.stopPropagation();

    const rack = racks.find(r => r.id === rackId);
    if (!rack || !draggingDevice) {
      handleDragEnd();
      return;
    }

    const deviceModel = getDeviceModel(draggingDevice.device_model_id);
    const deviceHeight = resolveDeviceHeight(draggingDevice, deviceModel);

    if (!deviceModel && deviceHeight === 1) {
      message.warning(`设备「${draggingDevice.name}」无型号且无历史高度，按 1U 占用`);
    }

    const existingDevices = getDevicesForRack(rackId)
      .filter(d => d.id !== draggingDevice.id)
      .map(d => ({ start_u: d.start_u, end_u: d.end_u, id: d.id }));

    const slot = findAvailableSlot(u, deviceHeight, rack.height_u, existingDevices);
    if (!slot) {
      message.warning(
        `机柜「${rack.name}」U 位不足：设备需要 ${deviceHeight}U，但目标位置附近没有足够连续空位。`
      );
      handleDragEnd();
      return;
    }

    const prev = {
      rack_id: draggingDevice.rack_id,
      start_u: draggingDevice.start_u,
      end_u: draggingDevice.end_u,
      height_u: draggingDevice.height_u,
      status: draggingDevice.status,
    };
    const label = draggingDevice.rack_id == null
      ? `上架设备「${draggingDevice.name}」`
      : `移动设备「${draggingDevice.name}」`;

    await update(draggingDevice.id, {
      rack_id: rackId,
      start_u: slot.startU,
      end_u: slot.endU,
      height_u: deviceHeight,
      ...(draggingDevice.status === 'unconfigured' ? { status: 'offline' as const } : {}),
    });

    // N-11 撤销：用 Patch 逆操作回填原 U 位/机柜
    pushUndo({
      label,
      undo: async () => { await update(draggingDevice.id, prev); },
    });

    handleDragEnd();
  }, [draggingDevice, racks, update, handleDragEnd, getDevicesForRack, getDeviceModel, resolveDeviceHeight, message, pushUndo]);

  const handleStockDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    if (draggingDevice?.rack_id != null) {
      e.dataTransfer.dropEffect = 'move';
      setDragOverStock(true);
    }
  }, [draggingDevice]);

  const handleStockDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOverStock(false);
    const device = draggingDevice;
    handleDragEnd();
    if (!device || device.rack_id == null) return;

    // N-13 下架二次确认；N-11 撤销用 Patch 逆操作回填原 U 位
    modal.confirm({
      title: '确认下架',
      content: `确定要将设备「${device.name}」下架到资源池吗？其占用的 U 位将被释放。`,
      okText: '下架',
      cancelText: '取消',
      onOk: async () => {
        const prev = { rack_id: device.rack_id, start_u: device.start_u, end_u: device.end_u };
        await update(device.id, {
          rack_id: null,
          start_u: null,
          end_u: null,
        });
        pushUndo({
          label: `下架设备「${device.name}」`,
          undo: async () => { await update(device.id, prev); },
        });
      },
    });
  }, [draggingDevice, update, handleDragEnd, modal, pushUndo]);

  const handleRemoveRack = async (rackId: number) => {
    const rack = racks.find(r => r.id === rackId);
    modal.confirm({
      title: '确认删除',
      content: `确定要删除机柜「${rack?.name || ''}」吗？其中的设备将保留但不再关联此机柜。`,
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        await removeRack(rackId);
        if (selectedRackId === rackId) {
          setSelectedRackId(null);
        }
      },
    });
  };

  const moveRackByOne = async (rack: Rack, dir: -1 | 1) => {
    const sameViewRacks = racks
      .filter(r => r.view === view)
      .sort((a, b) => a.sort_order - b.sort_order);
    const idx = sameViewRacks.findIndex(r => r.id === rack.id);
    const target = dir === -1 ? sameViewRacks[idx - 1] : sameViewRacks[idx + 1];
    if (!target) return;
    await updateRackQuiet(rack.id, { sort_order: target.sort_order });
    await updateRackQuiet(target.id, { sort_order: rack.sort_order });
    refreshRacks();
  };

  const handleMoveRackLeft = (rack: Rack) => moveRackByOne(rack, -1);
  const handleMoveRackRight = (rack: Rack) => moveRackByOne(rack, 1);

  /** 导出全部机柜部署图（Excel，Rust 侧弹保存框） */
  const handleExportRackPlan = useCallback(async () => {
    try {
      await tauriApi.exportRacksExcel();
    } catch (error) {
      console.error('导出机柜部署图失败:', error);
      message.error('导出机柜部署图失败');
    }
  }, [message]);

  /** 导出当前选中机柜部署图 */
  const handleExportSingleRack = useCallback(async () => {
    if (selectedRackId == null) return;
    try {
      await tauriApi.exportSingleRackExcel(selectedRackId);
    } catch (error) {
      console.error('导出单机柜失败:', error);
      message.error('导出单机柜失败');
    }
  }, [selectedRackId, message]);

  const handleSidebarDeviceClick = (device: Device) => {
    setSelectedDevice(device);
    setSelectedRackId(null);
  };

  const selectedDeviceInfo: SelectedDeviceInfo | null = selectedDevice ? {
    device: selectedDevice,
    model: getDeviceModel(selectedDevice.device_model_id),
    rack: racks.find(r => r.id === selectedDevice.rack_id),
  } : null;

  // ---------- N-11：详情面板直接透传的 update / remove 包一层，登记撤销 ----------
  const updateWithUndo = useCallback(async (id: number, data: Partial<Device>) => {
    const prev = devices.find(d => d.id === id);
    await update(id, data);
    if (prev) {
      const inverse: Partial<Device> = {};
      Object.keys(data).forEach(k => {
        (inverse as Record<string, unknown>)[k] = (prev as unknown as Record<string, unknown>)[k];
      });
      pushUndo({ label: `编辑设备「${prev.name}」`, undo: async () => { await update(id, inverse); } });
    }
  }, [devices, update, pushUndo]);

  const removeWithUndo = useCallback(async (id: number) => {
    const prev = devices.find(d => d.id === id);
    await remove(id);
    if (prev) {
      pushUndo({
        label: `删除设备「${prev.name}」`,
        undo: async () => { await tauriApi.restoreDevice(id); refreshDevices(); },
      });
    }
  }, [devices, remove, pushUndo, refreshDevices]);

  /** N-12：Delete —— 删除当前选中设备（二次确认；无选中则无副作用） */
  const handleDeleteSelectedDevice = useCallback(() => {
    const device = selectedDevice;
    if (!device) return;
    modal.confirm({
      title: '确认删除',
      content: `确定要删除设备「${device.name}」吗？删除后可在 30 天内在「回收站」恢复。`,
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        await removeWithUndo(device.id);
        setSelectedDevice(null);
      },
    });
  }, [selectedDevice, modal, removeWithUndo]);

  // N-12 全局快捷键（RackView 上下文）：Ctrl+Z 撤销 / Delete 删除选中 / F5 刷新
  useRegisterShortcuts({
    onUndo: () => { void undo(); },
    onDelete: handleDeleteSelectedDevice,
    onRefresh: () => { refreshDevices(); refreshRacks(); },
  });

  return {
    // context / hooks
    view, zoom, onZoomIn, onZoomOut, onZoomReset,
    searchQuery, onSearchChange, showAddRack, setShowAddRack,
    rooms, selectedRoomId,
    racks, models,
    // 派生数据
    filteredRacks, getDevicesForRack, getDeviceModel, getRoomName,
    unassignedDevices, rackStats, displayStats, searchMatchedDeviceIds,
    // 选中/拖拽状态
    selectedRackId, selectedDevice, draggingDevice, dropTarget, dragOverStock,
    hoveredDeviceId, deviceDetailVisible, detailDevice, rackEditVisible, editingRack,
    selectedDeviceInfo,
    // handlers
    handleRackClick, handleDeviceClick, handleDeviceDoubleClick, handleRackDoubleClick,
    handleAddRackSubmit, handleDetailDeviceSave, handleRackEditSave,
    handleDragStart, handleDragEnd, handleDragOver, handleDragLeave, handleDrop,
    handleStockDragOver, handleStockDrop,
    handleRemoveRack,
    handleMoveRackLeft, handleMoveRackRight,
    handleSidebarDeviceClick,
    handleExportRackPlan, handleExportSingleRack,
    // 直接回调透传（详情面板经此登记撤销）
    update: updateWithUndo, remove: removeWithUndo,
    closeDeviceDetail: () => { setDeviceDetailVisible(false); setDetailDevice(null); },
    closeRackEdit: () => { setRackEditVisible(false); setEditingRack(null); },
    clearSelectedDevice: () => setSelectedDevice(null),
    setHoveredDeviceId,
    addRack: () => setShowAddRack(true),
  };
}

export type UseRackViewReturn = ReturnType<typeof useRackView>;
