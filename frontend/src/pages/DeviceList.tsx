import { useState, useMemo, useEffect, useCallback } from 'react';
import type { Key } from 'react';
import { Table, Button, Input, Popover, Checkbox, App, Dropdown } from 'antd';
import { PlusOutlined, SettingOutlined, ColumnHeightOutlined, UploadOutlined, ExportOutlined, ReloadOutlined, DeleteOutlined } from '@ant-design/icons';
import { useDevices } from '../hooks/useDevices';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { useRacks } from '../hooks/useRacks';
import { useRooms } from '../hooks/useRooms';
import { Device } from '../types';
import { useRoomContext } from '../contexts/RoomContext';
import * as tauriApi from '../tauri-api';
import DeviceFormModal from '../components/device/DeviceFormModal';
import ModelManageModal from '../components/device/ModelManageModal';
import TrashDrawer from '../components/device/TrashDrawer';
import ImportWizardModal from '../components/device/ImportWizardModal';
import ResizableTitle from '../components/device/ResizableTitle';
import { ALL_COLUMNS, DEFAULT_COLUMN_WIDTHS, buildDeviceColumns, DeviceColumnKey } from '../components/device/deviceColumns';

/** N-20：单次批量删除上限（与后端 1000 校验一致） */
const MAX_BATCH_DELETE = 1000;

export default function DeviceList() {
  const { devices, loading, refresh, remove, create, update, removeMany } = useDevices();
  const { models, create: createModel, update: updateModel, remove: removeModel } = useDeviceModels();
  const { racks } = useRacks();
  const { rooms } = useRooms();
  const { selectedRoomId } = useRoomContext();
  const { modal, message } = App.useApp();

  const [currentPage, setCurrentPage] = useState(1);
  const [pageSize, setPageSize] = useState(10);

  const roomRackIds = useMemo(() => {
    if (selectedRoomId === null) return null;
    return new Set(racks.filter(r => r.room_id === selectedRoomId).map(r => r.id));
  }, [racks, selectedRoomId]);
  const [searchText, setSearchText] = useState('');
  const [deviceModalVisible, setDeviceModalVisible] = useState(false);
  const [editingDevice, setEditingDevice] = useState<Device | null>(null);

  const [modelModalVisible, setModelModalVisible] = useState(false);
  const [trashOpen, setTrashOpen] = useState(false);
  const [importWizardOpen, setImportWizardOpen] = useState(false);

  // N-20：选区（以 id 集合维护，preserveSelectedRowKeys 支持跨页/排序/搜索保持）
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [batchDeleting, setBatchDeleting] = useState(false);

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

  useEffect(() => {
    setCurrentPage(1);
  }, [searchText, selectedRoomId]);

  const showDeviceModal = useCallback((device?: Device) => {
    setEditingDevice(device ?? null);
    setDeviceModalVisible(true);
  }, []);

  const handleDeviceSave = async (payload: Record<string, unknown>, editing: Device | null) => {
    try {
      if (editing) {
        await update(editing.id, payload as Partial<Device>);
      } else {
        await create(payload as Partial<Device>);
      }
      setDeviceModalVisible(false);
      setEditingDevice(null);
      message.success(editing ? '设备已更新' : '设备已添加');
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

  // 单条删除：保持既有乐观更新语义（N-09 已改为软删，提示文案相应更新）
  const handleDeleteDevice = useCallback((device: Device) => {
    modal.confirm({
      title: '确认删除',
      content: `确定要删除设备「${device.name}」吗？删除后可在 30 天内在「回收站」恢复。`,
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: () => remove(device.id),
    });
  }, [modal, remove]);

  // ---------- N-20 批量删除：选区派生 ----------
  const selectedDevices = useMemo(
    () => selectedRowKeys
      .map(k => devices.find(d => d.id === Number(k)))
      .filter((d): d is Device => Boolean(d)),
    [selectedRowKeys, devices],
  );
  const onsiteSelectedCount = useMemo(
    () => selectedDevices.filter(d => d.rack_id != null).length,
    [selectedDevices],
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
    const onsiteCount = selectedDevices.filter(d => d.rack_id != null).length;

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
          const result = await removeMany(ids);
          // ⑧ 成功后清空选区
          setSelectedRowKeys([]);
          if (result.not_found.length > 0) {
            // ⑥ not_found 非空提示
            message.warning(`已删除 ${result.deleted} 台，${result.not_found.length} 台不存在已跳过`);
          } else {
            message.success(`已删除 ${result.deleted} 台设备`);
          }
        } catch (err) {
          // ⑦ 事务失败（后端整体回滚）：本地列表不变 + message.error（不 rethrow → 弹窗关闭）
          message.error(`批量删除失败：${tauriApi.errorMessage(err)}`);
        } finally {
          setBatchDeleting(false);
        }
      },
    });
  }, [selectedRowKeys, selectedDevices, modal, message, removeMany]);

  const rowSelection = {
    selectedRowKeys,
    preserveSelectedRowKeys: true,
    onChange: (keys: Key[]) => setSelectedRowKeys(keys),
  };

  const filteredDevices = useMemo(() => devices.filter(d => {
    if (!d.name.toLowerCase().includes(searchText.toLowerCase())) return false;
    if (roomRackIds === null) return true;
    if (d.rack_id === null) return false;
    return roomRackIds.has(d.rack_id);
  }), [devices, searchText, roomRackIds]);

  const allDeviceColumns = useMemo(
    () => buildDeviceColumns(models, racks, rooms, {
      onEdit: showDeviceModal,
      onDelete: handleDeleteDevice,
    }),
    [models, racks, rooms, showDeviceModal, handleDeleteDevice],
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
            className="device-list-search"
            placeholder="搜索设备..."
            value={searchText}
            onChange={e => setSearchText(e.target.value)}
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
            <Button type="primary" danger loading={batchDeleting} disabled={batchDeleting} onClick={handleBatchDelete}>
              批量删除
            </Button>
            <Button disabled={batchDeleting} onClick={() => setSelectedRowKeys([])}>
              取消选择
            </Button>
          </div>
        </div>
      )}

      <div className="device-list-table-wrap">
        <Table
          dataSource={filteredDevices}
          columns={visibleDeviceColumns}
          components={components}
          loading={loading}
          rowKey="id"
          rowSelection={rowSelection}
          pagination={{
            current: currentPage,
            pageSize,
            pageSizeOptions: [10, 20, 50],
            showSizeChanger: true,
            showTotal: (total: number) => `共 ${total} 条`,
            onChange: (page, size) => {
              setCurrentPage(page);
              setPageSize(size);
            },
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
    </div>
  );
}
