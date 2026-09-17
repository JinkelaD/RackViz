import { useState, useMemo, useEffect, useCallback, useRef } from 'react';
import type { Key } from 'react';
import { useNavigate } from 'react-router-dom';
import { Table, Button, Input, Popover, Checkbox, App, Dropdown } from 'antd';
import type { TableProps, InputRef } from 'antd';
import { PlusOutlined, SettingOutlined, ColumnHeightOutlined, UploadOutlined, ExportOutlined, ReloadOutlined, DeleteOutlined, PrinterOutlined, FormOutlined } from '@ant-design/icons';
import { usePagedDevices } from '../hooks/usePagedDevices';
import { useDevices } from '../hooks/useDevices';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { useRacks } from '../hooks/useRacks';
import { useRooms } from '../hooks/useRooms';
import { Device, DeviceSortField } from '../types';
import { useRoomContext } from '../contexts/RoomContext';
import { useViewContext } from '../contexts/ViewContext';
import { useUndo } from '../contexts/UndoContext';
import { useRegisterShortcuts } from '../hooks/useGlobalShortcuts';
import { pickFields } from '../utils/objectFields';
import * as tauriApi from '../tauri-api';
import DeviceFormModal from '../components/device/DeviceFormModal';
import ModelManageModal from '../components/device/ModelManageModal';
import TrashDrawer from '../components/device/TrashDrawer';
import ImportWizardModal from '../components/device/ImportWizardModal';
import BatchPrintModal from '../components/device/BatchPrintModal';
import BatchEditModal from '../components/device/BatchEditModal';
import ResizableTitle from '../components/device/ResizableTitle';
import { ALL_COLUMNS, DEFAULT_COLUMN_WIDTHS, buildDeviceColumns, DeviceColumnKey, isSortField } from '../components/device/deviceColumns';

/** N-20：单次批量删除上限（与后端 1000 校验一致） */
const MAX_BATCH_DELETE = 1000;

export default function DeviceList() {
  const navigate = useNavigate();
  const { selectedRoomId, setSelectedRoomId } = useRoomContext();
  const { onSearchChange } = useViewContext();
  const { models, create: createModel, update: updateModel, remove: removeModel } = useDeviceModels();
  const { racks } = useRacks();
  const { rooms } = useRooms();
  const { modal, message } = App.useApp();
  const { pushUndo, undo } = useUndo();
  const searchInputRef = useRef<InputRef>(null);

  // T3.2：服务端分页 + 多字段搜索 + 排序（机房过滤经 room_id 下推服务端）
  const { items, total, loading, query, setQuery, refresh } = usePagedDevices({
    room_id: selectedRoomId,
    limit: 10,
  });

  const [deviceModalVisible, setDeviceModalVisible] = useState(false);
  const [editingDevice, setEditingDevice] = useState<Device | null>(null);

  const [modelModalVisible, setModelModalVisible] = useState(false);
  const [trashOpen, setTrashOpen] = useState(false);
  const [importWizardOpen, setImportWizardOpen] = useState(false);

  // N-20：选区（以 id 集合维护，preserveSelectedRowKeys 支持跨页/排序/搜索保持）
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  // 已选设备对象缓存（跨页时用于「在架」计数；onChange 的 second 参数仅含当前页选中行）
  const [selectedDeviceMap, setSelectedDeviceMap] = useState<Record<number, Device>>({});
  const [batchDeleting, setBatchDeleting] = useState(false);
  // B4：二维码标签批量打印（单选/多选统一走批量通道）
  const [batchPrintOpen, setBatchPrintOpen] = useState(false);
  // B2：批量编辑（机柜/U 位/状态）
  const [batchEditOpen, setBatchEditOpen] = useState(false);
  // B2 自动排布需全量设备算目标机柜占用图（本地 IPC，开销可忽略）
  const { devices: allDevices } = useDevices();

  const [visibleColumns, setVisibleColumns] = useState<DeviceColumnKey[]>(
    ALL_COLUMNS.map(c => c.key)
  );

  const [columnWidths, setColumnWidths] = useState<Record<DeviceColumnKey, number>>(
    () => ALL_COLUMNS.reduce((acc, c) => {
      acc[c.key] = DEFAULT_COLUMN_WIDTHS[c.key] || 100;
      return acc;
    }, {} as Record<DeviceColumnKey, number>)
  );

  const handleColumnResize = useCallback((key: DeviceColumnKey) => (width: number) => {
    setColumnWidths(prev => ({ ...prev, [key]: width }));
  }, []);

  // 切机房 → 下推 room_id（服务端过滤，与搜索叠加）；跳过首帧避免与初始查询重复取数
  const firstRoomSync = useRef(true);
  useEffect(() => {
    if (firstRoomSync.current) {
      firstRoomSync.current = false;
      return;
    }
    setQuery({ room_id: selectedRoomId });
  }, [selectedRoomId, setQuery]);

  const showDeviceModal = useCallback((device?: Device) => {
    setEditingDevice(device ?? null);
    setDeviceModalVisible(true);
  }, []);

  const handleDeviceSave = async (payload: Record<string, unknown>, editing: Device | null) => {
    try {
      if (editing) {
        // N-11：先按 payload 字段快照原值，作为逆操作回填
        const inverse: Partial<Device> = {};
        Object.assign(inverse, pickFields(editing, Object.keys(payload)));
        await tauriApi.updateDevice(editing.id, payload as Partial<Device>);
        pushUndo({
          label: `编辑设备「${editing.name}」`,
          undo: async () => { await tauriApi.updateDevice(editing.id, inverse); refresh(); },
        });
      } else {
        const created = await tauriApi.createDevice(payload as unknown as tauriApi.DeviceCreate);
        pushUndo({
          label: `新建设备「${created.name}」`,
          undo: async () => { await tauriApi.deleteDevice(created.id); refresh(); },
        });
      }
      setDeviceModalVisible(false);
      setEditingDevice(null);
      message.success(editing ? '设备已更新' : '设备已添加');
      refresh();
    } catch (error) {
      console.error('保存设备失败:', error);
      message.error('保存设备失败，请重试');
      throw error; // 让弹窗保持打开，用户可修正
    }
  };

  const handleDeviceCancel = () => {
    setDeviceModalVisible(false);
    setEditingDevice(null);
  };

  const handleModelModalClose = () => {
    setModelModalVisible(false);
  };

  // B1 全局搜索「结果直达定位」：切换到设备所在机房 → 填入机柜视图搜索词（触发高亮）→ 路由跳转。
  // RoomContext / ViewContext 均为 Layout 级 Provider，跨路由状态保留。
  const handleLocate = useCallback((device: Device) => {
    const rack = racks.find(r => r.id === device.rack_id);
    if (!rack) {
      message.warning('设备所在机柜不存在，无法定位');
      return;
    }
    const roomName = rooms.find(rm => rm.id === rack.room_id)?.name;
    setSelectedRoomId(rack.room_id);
    onSearchChange(device.name);
    navigate('/racks');
    message.success(`已定位到${roomName ? `「${roomName}」` : '未分配机房'} · 机柜「${rack.name}」`);
  }, [racks, rooms, setSelectedRoomId, onSearchChange, navigate, message]);

  // 单条删除（N-09 软删除；非乐观：成功后重取当前页）
  const handleDeleteDevice = useCallback((device: Device) => {
    modal.confirm({
      title: '确认删除',
      content: `确定要删除设备「${device.name}」吗？删除后可在 30 天内在「回收站」恢复。`,
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        try {
          await tauriApi.deleteDevice(device.id);
          refresh();
          pushUndo({
            label: `删除设备「${device.name}」`,
            undo: async () => { await tauriApi.restoreDevice(device.id); refresh(); },
          });
        } catch (err) {
          message.error(`删除失败：${tauriApi.errorMessage(err)}`);
        }
      },
    });
  }, [modal, message, refresh, pushUndo]);

  // ---------- N-20 批量删除：选区派生 ----------
  const onsiteSelectedCount = useMemo(
    () => selectedRowKeys
      .map(k => selectedDeviceMap[Number(k)])
      .filter(d => d && d.rack_id != null).length,
    [selectedRowKeys, selectedDeviceMap],
  );

  const handleBatchDelete = useCallback(() => {
    // ① 空选区拦截（操作条仅在选区非空时渲染，此处兜底）
    // ② 前端 id 去重
    const ids = Array.from(new Set(selectedRowKeys.map(Number)));
    if (ids.length === 0) {
      message.warning('请先选择要删除的设备');
      return;
    }
    // ③ 上限 1000 拦截
    if (ids.length > MAX_BATCH_DELETE) {
      message.error(`单次批量删除最多 ${MAX_BATCH_DELETE} 台，请分批操作`);
      return;
    }
    // ④ 在架设备计数警告
    const onsiteCount = ids
      .map(id => selectedDeviceMap[id])
      .filter(d => d && d.rack_id != null).length;

    modal.confirm({
      title: '确认批量删除',
      content: (
        <div className="batch-delete-confirm">
          <p>确定要删除选中的 {ids.length} 台设备吗？</p>
          {onsiteCount > 0 && (
            <p className="batch-delete-warn">⚠️ 其中 {onsiteCount} 台在架，删除后对应 U 位将被释放。</p>
          )}
          <p>删除后可在 30 天内在「回收站」恢复；超期不可恢复。</p>
        </div>
      ),
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        // ⑤ 提交中禁用防重复（submitting）
        setBatchDeleting(true);
        try {
          const result = await tauriApi.deleteDevices(ids);
          // ⑧ 成功后清空选区
          setSelectedRowKeys([]);
          setSelectedDeviceMap({});
          if (result.not_found.length > 0) {
            // ⑥ not_found 非空提示
            message.warning(`已删除 ${result.deleted} 台，${result.not_found.length} 台不存在已跳过`);
          } else {
            message.success(`已删除 ${result.deleted} 台设备`);
          }
          refresh();
          // N-11 撤销：恢复本次成功删除的设备（跳过 not_found）
          const deletedIds = ids.filter(id => !result.not_found.includes(id));
          if (deletedIds.length > 0) {
            pushUndo({
              label: `批量删除 ${deletedIds.length} 台设备`,
              undo: async () => {
                for (const id of deletedIds) {
                  await tauriApi.restoreDevice(id);
                }
                refresh();
              },
            });
          }
        } catch (err) {
          // ⑦ 事务失败（后端整体回滚）：本地列表不变 + message.error（不 rethrow → 弹窗关闭）
          message.error(`批量删除失败：${tauriApi.errorMessage(err)}`);
        } finally {
          setBatchDeleting(false);
        }
      },
    });
  }, [selectedRowKeys, selectedDeviceMap, modal, message, refresh, pushUndo]);

  const rowSelection = {
    selectedRowKeys,
    preserveSelectedRowKeys: true,
    onChange: (keys: Key[], rows: Device[]) => {
      setSelectedRowKeys(keys);
      // 累积已知选中行对象（供跨页「在架」计数），并剔除已取消项
      setSelectedDeviceMap(prev => {
        const next: Record<number, Device> = { ...prev };
        rows.forEach(r => { next[r.id] = r; });
        const keySet = new Set(keys.map(Number));
        Object.keys(next).forEach(k => {
          if (!keySet.has(Number(k))) delete next[Number(k)];
        });
        return next;
      });
    },
  };

  // 服务端分页 + 排序：由 pagination/sorter 变化驱动 query
  const handleTableChange: TableProps<Device>['onChange'] = (pag, _filters, sorter) => {
    const s = Array.isArray(sorter) ? sorter[0] : sorter;
    const rawField = s?.field;
    const field = isSortField(rawField) ? rawField : undefined;
    const order: 'asc' | 'desc' | null = s?.order === 'ascend' ? 'asc' : s?.order === 'descend' ? 'desc' : null;
    const nextSortField = order && field ? field : null;

    const sortChanged =
      nextSortField !== (query.sort_field ?? null) || order !== (query.sort_order ?? null);
    const size = pag.pageSize ?? query.limit ?? 10;

    setQuery({
      limit: size,
      // 排序变化时回到第一页；否则按目标页计算 offset
      offset: sortChanged ? 0 : ((pag.current ?? 1) - 1) * size,
      sort_field: nextSortField,
      sort_order: order,
    });
  };

  const allDeviceColumns = useMemo(
    () => buildDeviceColumns(models, racks, rooms, {
      onEdit: showDeviceModal,
      onDelete: handleDeleteDevice,
      onLocate: handleLocate,
    }, {
      search: query.search ?? '',
      sortField: query.sort_field ?? null,
      sortOrder: query.sort_order ?? null,
    }),
    [models, racks, rooms, showDeviceModal, handleDeleteDevice, handleLocate, query.search, query.sort_field, query.sort_order],
  );

  const visibleDeviceColumns = allDeviceColumns
    .filter(col => visibleColumns.includes(col.key as DeviceColumnKey))
    .map(col => ({
      ...col,
      width: columnWidths[col.key as DeviceColumnKey] || 100,
      onHeaderCell: (column: typeof col) => ({
        width: (column as { width?: number }).width,
        onResize: handleColumnResize(col.key as DeviceColumnKey),
      }),
    }));

  const components = {
    header: {
      cell: ResizableTitle,
    },
  };

  const columnVisibilityContent = (
    <Checkbox.Group
      value={visibleColumns}
      onChange={vals => setVisibleColumns(vals as DeviceColumnKey[])}
      style={{ display: 'flex', flexDirection: 'column', gap: 4 }}
    >
      {ALL_COLUMNS.map(c => (
        <Checkbox key={c.key} value={c.key}>{c.title}</Checkbox>
      ))}
    </Checkbox.Group>
  );

  // N-12 全局快捷键（台账上下文）：Ctrl+N 新增 / Ctrl+F 聚焦搜索 / Ctrl+Z 撤销 / Delete 删除选中 / F5 刷新
  useRegisterShortcuts({
    onNew: () => showDeviceModal(),
    onSearch: () => searchInputRef.current?.focus(),
    onUndo: () => { void undo(); },
    onDelete: () => { if (selectedRowKeys.length > 0) handleBatchDelete(); },
    onRefresh: refresh,
  });

  return (
    <div className="device-list-page">
      <div className="device-list-header">
        <h2 className="device-list-title">设备台账</h2>
        <div className="device-list-toolbar">
          <Popover content={columnVisibilityContent} title="选择显示的列" trigger="click" placement="bottomRight">
            <Button icon={<ColumnHeightOutlined />}>列选项</Button>
          </Popover>
          <Button onClick={() => setModelModalVisible(true)} icon={<SettingOutlined />}>
            型号管理
          </Button>
          <Button icon={<UploadOutlined />} onClick={() => setImportWizardOpen(true)}>
              导入
          </Button>
          <Button icon={<DeleteOutlined />} onClick={() => setTrashOpen(true)}>
              回收站
          </Button>
          <Dropdown
            menu={{
              items: [
                { key: 'excel', label: 'Excel 格式', onClick: async () => {
                  await tauriApi.exportDevicesDataExcel();
                }},
                { key: 'html', label: 'HTML 格式', onClick: async () => {
                  await tauriApi.exportReportHtml();
                }},
              ],
            }}
            trigger={['click']}
          >
            <Button icon={<ExportOutlined />}>导出</Button>
          </Dropdown>
          <Button type="primary" onClick={() => showDeviceModal()} icon={<PlusOutlined />}>
            添加设备
          </Button>
          <Button icon={<ReloadOutlined />} onClick={refresh} loading={loading}>
            刷新
          </Button>
          <Input
            ref={searchInputRef}
            className="device-list-search"
            placeholder="搜索名称 / IP / 序列号 / 资产编号 / 使用人..."
            value={query.search ?? ''}
            onChange={e => setQuery({ search: e.target.value })}
          />
        </div>
      </div>

      {/* N-20 选区操作条：仅在有选区时渲染，无选区不占位 */}
      {selectedRowKeys.length > 0 && (
        <div className="device-list-selection-bar">
          <span className="selection-info">已选 <b>{selectedRowKeys.length}</b> 台</span>
          {onsiteSelectedCount > 0 && (
            <span className="selection-onsite">其中 {onsiteSelectedCount} 台在架</span>
          )}
          <div className="selection-actions">
            <Button icon={<FormOutlined />} onClick={() => setBatchEditOpen(true)}>
              批量编辑
            </Button>
            <Button icon={<PrinterOutlined />} onClick={() => setBatchPrintOpen(true)}>
              打印标签
            </Button>
            <Button type="primary" danger loading={batchDeleting} disabled={batchDeleting} onClick={handleBatchDelete}>
              批量删除
            </Button>
            <Button disabled={batchDeleting} onClick={() => { setSelectedRowKeys([]); setSelectedDeviceMap({}); }}>
              取消选择
            </Button>
          </div>
        </div>
      )}

      <div className="device-list-table-wrap">
        <Table
          dataSource={items}
          columns={visibleDeviceColumns}
          components={components}
          loading={loading}
          rowKey="id"
          rowSelection={rowSelection}
          onChange={handleTableChange}
          pagination={{
            current: Math.floor((query.offset ?? 0) / (query.limit ?? 10)) + 1,
            pageSize: query.limit ?? 10,
            total,
            pageSizeOptions: [10, 20, 50],
            showSizeChanger: true,
            showTotal: (t: number) => `共 ${t} 条`,
          }}
          bordered={false}
          scroll={{ x: 'max-content' }}
          size="middle"
        />
      </div>

      <DeviceFormModal
        open={deviceModalVisible}
        editing={editingDevice}
        models={models}
        racks={racks}
        onCancel={handleDeviceCancel}
        onSave={handleDeviceSave}
      />

      <ModelManageModal
        open={modelModalVisible}
        models={models}
        onClose={handleModelModalClose}
        createModel={createModel}
        updateModel={updateModel}
        removeModel={removeModel}
      />

      <TrashDrawer
        open={trashOpen}
        onClose={() => setTrashOpen(false)}
        onRestored={refresh}
      />

      <ImportWizardModal
        open={importWizardOpen}
        onClose={() => setImportWizardOpen(false)}
        onImported={refresh}
      />

      <BatchEditModal
        open={batchEditOpen}
        devices={selectedRowKeys.map(k => selectedDeviceMap[Number(k)]).filter(Boolean)}
        allDevices={allDevices}
        racks={racks}
        rooms={rooms}
        refresh={refresh}
        onCancel={() => setBatchEditOpen(false)}
        onDone={() => { setBatchEditOpen(false); refresh(); }}
      />

      <BatchPrintModal
        open={batchPrintOpen}
        devices={selectedRowKeys.map(k => selectedDeviceMap[Number(k)]).filter(Boolean)}
        models={models}
        racks={racks}
        rooms={rooms}
        onClose={() => setBatchPrintOpen(false)}
      />
    </div>
  );
}
