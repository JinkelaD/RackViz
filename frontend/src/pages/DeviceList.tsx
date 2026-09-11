import { useState, useMemo, useEffect, useCallback } from 'react';
import { Table, Button, Input, Popover, Checkbox, App, Dropdown } from 'antd';
import { PlusOutlined, SettingOutlined, ColumnHeightOutlined, UploadOutlined, ExportOutlined, ReloadOutlined } from '@ant-design/icons';
import { useDevices } from '../hooks/useDevices';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { useRacks } from '../hooks/useRacks';
import { useRooms } from '../hooks/useRooms';
import { Device } from '../types';
import { useRoomContext } from '../contexts/RoomContext';
import * as tauriApi from '../tauri-api';
import DeviceFormModal from '../components/device/DeviceFormModal';
import ModelManageModal from '../components/device/ModelManageModal';
import ResizableTitle from '../components/device/ResizableTitle';
import { ALL_COLUMNS, DEFAULT_COLUMN_WIDTHS, buildDeviceColumns, DeviceColumnKey } from '../components/device/deviceColumns';

export default function DeviceList() {
  const { devices, loading, refresh, remove, create, update } = useDevices();
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

  const [visibleColumns, setVisibleColumns] = useState<DeviceColumnKey[]>(
    ALL_COLUMNS.map(c => c.key)
  );
  const [importing, setImporting] = useState(false);

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

  const handleDeviceImport = async () => {
    setImporting(true);
    try {
      const result = await tauriApi.importExcelFromPath();
      const parts = [`成功 ${result.imported} 条`];
      if (result.skipped > 0) parts.push(`跳过重复 ${result.skipped} 条`);
      if (result.errors.length > 0) parts.push(`失败 ${result.errors.length} 条`);
      if (result.errors.length > 0) {
        message.warning(`导入完成：${parts.join('，')}`);
      } else if (result.skipped > 0) {
        message.info(`导入完成：${parts.join('，')}`);
      } else {
        message.success(`成功导入 ${result.imported} 条设备`);
      }
      refresh();
    } catch {
      message.error('导入失败，请检查文件格式');
    } finally {
      setImporting(false);
    }
  };

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

  const handleDeleteDevice = useCallback((device: Device) => {
    modal.confirm({
      title: '确认删除',
      content: `确定要删除设备「${device.name}」吗？此操作不可撤销。`,
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: () => remove(device.id),
    });
  }, [modal, remove]);

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
          <Button icon={<UploadOutlined />} loading={importing} onClick={handleDeviceImport}>
              导入
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

      <div className="device-list-table-wrap">
        <Table
          dataSource={filteredDevices}
          columns={visibleDeviceColumns}
          components={components}
          loading={loading}
          rowKey="id"
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
    </div>
  );
}
