import { useState, useMemo, useEffect, useRef, useCallback } from 'react';
import { useOutletContext } from 'react-router-dom';
import { Table, Button, Input, Tag, Space, Modal, Form, Select, InputNumber, Popover, Checkbox, App, Dropdown } from 'antd';
import { EditOutlined, PlusOutlined, SettingOutlined, ColumnHeightOutlined, UploadOutlined, ExportOutlined, ReloadOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { useDevices } from '../hooks/useDevices';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { useRacks } from '../hooks/useRacks';
import { useRooms } from '../hooks/useRooms';
import { Device, DeviceModel } from '../types';
import * as tauriApi from '../tauri-api';
import { DEVICE_TYPE_LABELS } from '../constants/labels';

interface DeviceListContext {
  selectedRoomId: number | null;
}

const deviceTypeOptions = [
  { value: 'server', label: '服务器' },
  { value: 'switch', label: '交换机' },
  { value: 'router', label: '路由器' },
  { value: 'storage', label: '存储' },
  { value: 'security', label: '安全设备' },
];

type DeviceColumnKey = 'name' | 'model' | 'rack' | 'room' | 'position' | 'ip' | 'asset_no' | 'department' | 'owner' | 'status' | 'action';

const DEFAULT_COLUMN_WIDTHS: Record<string, number> = {
  name: 160,
  model: 160,
  room: 110,
  rack: 110,
  position: 90,
  ip: 140,
  asset_no: 130,
  department: 120,
  owner: 100,
  status: 90,
  action: 90,
};

function ResizableTitle(props: React.HTMLAttributes<HTMLElement> & {
  onResize: (width: number) => void;
  width?: number;
}) {
  const { onResize, width, style, ...restProps } = props;
  const [resizing, setResizing] = useState(false);
  const startX = useRef(0);
  const startW = useRef(0);

  const onMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setResizing(true);
    startX.current = e.clientX;
    startW.current = width || 100;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  };

  useEffect(() => {
    if (!resizing) return;
    const onMouseMove = (e: MouseEvent) => {
      const diff = e.clientX - startX.current;
      onResize(Math.max(50, startW.current + diff));
    };
    const onMouseUp = () => {
      setResizing(false);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
    return () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    };
  }, [resizing, onResize]);

  return (
    <th {...restProps} style={{ ...style, position: 'relative' }}>
      {restProps.children}
      <div
        className="column-resize-handle"
        onMouseDown={onMouseDown}
      />
    </th>
  );
}

const ALL_COLUMNS: { key: DeviceColumnKey; title: string }[] = [
  { key: 'name', title: '设备名称' },
  { key: 'model', title: '型号' },
  { key: 'room', title: '机房' },
  { key: 'rack', title: '机柜' },
  { key: 'position', title: '位置' },
  { key: 'ip', title: 'IP' },
  { key: 'asset_no', title: '资产编号' },
  { key: 'department', title: '使用部门' },
  { key: 'owner', title: '责任人' },
  { key: 'status', title: '状态' },
  { key: 'action', title: '操作' },
];

function getTypeTag(type: string) {
  const colors: Record<string, string> = {
    server: 'blue',
    switch: 'cyan',
    router: 'gold',
    storage: 'purple',
    security: 'orange',
  };
  return <Tag color={colors[type]}>{DEVICE_TYPE_LABELS[type] || type}</Tag>;
}

export default function DeviceList() {
  const { devices, loading, refresh, remove, create, update } = useDevices();
  const { models, create: createModel, update: updateModel, remove: removeModel } = useDeviceModels();
  const { racks } = useRacks();
  const { rooms } = useRooms();
  const { selectedRoomId } = useOutletContext<DeviceListContext>();
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
  const [deviceForm] = Form.useForm();

  const [modelModalVisible, setModelModalVisible] = useState(false);
  const [editingModel, setEditingModel] = useState<DeviceModel | null>(null);
  const [modelSearchText, setModelSearchText] = useState('');
  const [modelForm] = Form.useForm();

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

  const showDeviceModal = (device?: Device) => {
    if (device) {
      setEditingDevice(device);
      deviceForm.setFieldsValue({
        ...device,
        device_model_id: device.device_model_id,
        rack_id: device.rack_id,
      });
    } else {
      setEditingDevice(null);
      deviceForm.resetFields();
    }
    setDeviceModalVisible(true);
  };

  const handleDeviceOk = async () => {
    try {
      const values = await deviceForm.validateFields();
      const payload = {
        ...values,
        rack_id: values.status === 'unconfigured' ? null : (values.rack_id ?? null),
        start_u: values.status === 'unconfigured' ? null : values.start_u,
        end_u: values.status === 'unconfigured' ? null : values.end_u,
      };
      if (editingDevice) {
        await update(editingDevice.id, payload);
      } else {
        await create(payload);
      }
      setDeviceModalVisible(false);
      deviceForm.resetFields();
      setEditingDevice(null);
    } catch (error) {
      console.error('Form validation failed:', error);
    }
  };

  const handleDeviceCancel = () => {
    setDeviceModalVisible(false);
    deviceForm.resetFields();
    setEditingDevice(null);
  };

  const handleModelChange = (modelId: number) => {
    const model = models.find(m => m.id === modelId);
    if (model) {
      deviceForm.setFieldsValue({ end_u: (deviceForm.getFieldValue('start_u') || 1) + model.height_u - 1 });
    }
  };

  const handleStartUChange = (startU: number | null) => {
    const modelId = deviceForm.getFieldValue('device_model_id');
    const model = models.find(m => m.id === modelId);
    if (model && startU) {
      deviceForm.setFieldsValue({ end_u: startU + model.height_u - 1 });
    }
  };

  const showModelModal = (model?: DeviceModel) => {
    if (model) {
      setEditingModel(model);
      modelForm.setFieldsValue(model);
    } else {
      setEditingModel(null);
      modelForm.resetFields();
    }
    setModelModalVisible(true);
  };

  const handleModelSubmit = (values: Record<string, unknown>) => {
    if (editingModel) {
      updateModel(editingModel.id, values).then(() => {
        setEditingModel(null);
        modelForm.resetFields();
      });
    } else {
      createModel(values).then(() => {
        modelForm.resetFields();
      });
    }
  };

  const getDeviceModelName = (modelId: number | null) => {
    if (modelId === null) return '-';
    return models.find(m => m.id === Number(modelId))?.name || '-';
  };

  const getRackName = (rackId: number | null) => {
    return racks.find(r => r.id === rackId)?.name || '-';
  };

  const getRoomName = (rackId: number | null) => {
    if (rackId === null) return '-';
    const rack = racks.find(r => r.id === rackId);
    if (!rack || rack.room_id === null) return '-';
    return rooms.find(rm => rm.id === rack.room_id)?.name || '-';
  };

  const getStatusTag = (status: string) => {
    const colors: Record<string, string> = {
      online: 'success',
      offline: 'error',
      unconfigured: 'default',
    };
    const labels: Record<string, string> = {
      online: '开机',
      offline: '离线',
      unconfigured: '未上架',
    };
    return <Tag color={colors[status]}>{labels[status]}</Tag>;
  };

  const filteredDevices = devices.filter(d => {
    if (!d.name.toLowerCase().includes(searchText.toLowerCase())) return false;
    if (roomRackIds === null) return true;
    if (d.rack_id === null) return false;
    return roomRackIds.has(d.rack_id);
  });

  const filteredModels = models.filter(m =>
    m.name.toLowerCase().includes(modelSearchText.toLowerCase())
  );

  const allDeviceColumns: ColumnsType<Device> = [
    { title: '设备名称', dataIndex: 'name', key: 'name', sorter: (a, b) => a.name.localeCompare(b.name, 'zh') },
    { title: '型号', dataIndex: 'device_model_id', key: 'model', render: (id: number | null) => getDeviceModelName(id), sorter: (a, b) => getDeviceModelName(a.device_model_id).localeCompare(getDeviceModelName(b.device_model_id), 'zh') },
    { title: '机房', dataIndex: 'rack_id', key: 'room', render: (id: number | null) => getRoomName(id), sorter: (a, b) => getRoomName(a.rack_id).localeCompare(getRoomName(b.rack_id), 'zh') },
    { title: '机柜', dataIndex: 'rack_id', key: 'rack', render: (id: number | null) => getRackName(id), sorter: (a, b) => getRackName(a.rack_id).localeCompare(getRackName(b.rack_id), 'zh') },
    { title: '位置', key: 'position', render: (_: unknown, record: Device) => record.start_u && record.end_u ? `${record.start_u}-${record.end_u}U` : '-', sorter: (a, b) => (a.start_u || 0) - (b.start_u || 0) },
    { title: 'IP', dataIndex: 'ip_addresses', key: 'ip', render: (ips: string) => ips || '-', sorter: (a, b) => (a.ip_addresses || '').localeCompare(b.ip_addresses || '') },
    { title: '资产编号', dataIndex: 'asset_no', key: 'asset_no', render: (no: string) => no || '-', sorter: (a, b) => (a.asset_no || '').localeCompare(b.asset_no || '') },
    { title: '使用部门', dataIndex: 'department', key: 'department', render: (dept: string) => dept || '-', sorter: (a, b) => (a.department || '').localeCompare(b.department || '', 'zh') },
    { title: '责任人', dataIndex: 'owner', key: 'owner', render: (owner: string) => owner || '-', sorter: (a, b) => (a.owner || '').localeCompare(b.owner || '', 'zh') },
    { title: '状态', dataIndex: 'status', key: 'status', render: (status: string) => getStatusTag(status), sorter: (a, b) => a.status.localeCompare(b.status) },
    {
      title: '操作', key: 'action', fixed: 'right' as const,
      render: (_: unknown, record: Device) => (
        <Space>
          <Button size="small" icon={<EditOutlined />} onClick={() => showDeviceModal(record)}>编辑</Button>
          <Button size="small" danger onClick={() => {
            modal.confirm({
              title: '确认删除',
              content: `确定要删除设备「${record.name}」吗？此操作不可撤销。`,
              okText: '删除',
              okType: 'danger',
              cancelText: '取消',
              onOk: () => remove(record.id),
            });
          }}>删除</Button>
        </Space>
      ),
    },
  ];

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

  const modelColumns = [
    { title: '型号', dataIndex: 'name', key: 'name', width: 180 },
    { title: '厂家', dataIndex: 'manufacturer', key: 'manufacturer', width: 120, render: (val: string) => val || '-' },
    { title: '类型', dataIndex: 'type', key: 'type', width: 90, render: (type: string) => getTypeTag(type) },
    { title: '高度', dataIndex: 'height_u', key: 'height_u', width: 70, render: (u: number) => `${u}U` },
    {
      title: '操作', key: 'action', width: 130,
      render: (_: unknown, record: DeviceModel) => (
        <Space>
          <Button size="small" onClick={() => showModelModal(record)}>编辑</Button>
          <Button size="small" danger onClick={() => removeModel(record.id)}>删除</Button>
        </Space>
      ),
    },
  ];

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
          <Button onClick={() => showModelModal()} icon={<SettingOutlined />}>
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

      <Modal
        title={editingDevice ? '编辑设备' : '添加设备'}
        open={deviceModalVisible}
        onOk={handleDeviceOk}
        onCancel={handleDeviceCancel}
        width={600}
      >
        <Form form={deviceForm} layout="vertical">
          <Form.Item name="name" label="设备名称" rules={[{ required: true, message: '请输入设备名称' }]}>
            <Input placeholder="请输入设备名称" />
          </Form.Item>
          <Form.Item name="device_model_id" label="设备型号" rules={[{ required: true, message: '请选择设备型号' }]}>
            <Select placeholder="请选择设备型号" onChange={handleModelChange}>
              {models.map(model => (
                <Select.Option key={model.id} value={model.id}>
                  {model.name} ({DEVICE_TYPE_LABELS[model.type] || model.type} · {model.height_u}U)
                </Select.Option>
              ))}
            </Select>
          </Form.Item>
          <Form.Item name="rack_id" label="所属机柜">
            <Select placeholder="请选择机柜（可选）" allowClear>
              {racks.map(rack => (
                <Select.Option key={rack.id} value={rack.id}>
                  {rack.name} ({rack.height_u}U)
                </Select.Option>
              ))}
            </Select>
          </Form.Item>
          <Form.Item name="status" label="设备状态">
            <Select placeholder="请选择状态">
              <Select.Option value="online">开机</Select.Option>
              <Select.Option value="offline">离线</Select.Option>
              <Select.Option value="unconfigured">未上架</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="start_u" label="起始U位">
            <InputNumber min={1} max={100} placeholder="起始U位" onChange={handleStartUChange} />
          </Form.Item>
          <Form.Item name="end_u" label="结束U位">
            <InputNumber min={1} max={100} placeholder="结束U位" disabled />
          </Form.Item>
          <Form.Item name="ip_addresses" label="IP地址">
            <Input placeholder="多个IP用逗号分隔" />
          </Form.Item>
          <Form.Item name="serial_no" label="序列号">
            <Input placeholder="设备序列号" />
          </Form.Item>
          <Form.Item name="asset_no" label="资产编号">
            <Input placeholder="资产编号（便于查账）" />
          </Form.Item>
          <Form.Item name="department" label="使用部门">
            <Input placeholder="所属部门" />
          </Form.Item>
          <Form.Item name="owner" label="责任人">
            <Input placeholder="设备负责人" />
          </Form.Item>
          <Form.Item name="function" label="功能描述">
            <Input placeholder="设备功能描述" />
          </Form.Item>
          <Form.Item name="purchase_date" label="采购日期">
            <Input type="date" />
          </Form.Item>
          <Form.Item name="warranty_expire" label="质保到期">
            <Input type="date" />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title="型号管理"
        open={modelModalVisible}
        onCancel={() => { setModelModalVisible(false); setEditingModel(null); modelForm.resetFields(); }}
        width={700}
        footer={null}
      >
        <div className="model-search-bar">
          <Input
            className="model-search-input"
            placeholder="搜索型号..."
            value={modelSearchText}
            onChange={e => setModelSearchText(e.target.value)}
          />
          <Button type="primary" onClick={() => { setEditingModel(null); modelForm.resetFields(); }}>
            新增型号
          </Button>
        </div>

        <Table
          dataSource={filteredModels}
          columns={modelColumns}
          rowKey="id"
          pagination={{ pageSize: 8 }}
          bordered={false}
          size="small"
        />

        <div className="model-form-header">
          <h4 className="model-form-title">
            {editingModel ? '编辑型号' : '新增型号'}
          </h4>
          <Form form={modelForm} layout="inline" onFinish={handleModelSubmit} style={{ gap: 8, flexWrap: 'wrap' }}>
            <Form.Item name="name" label="名称" rules={[{ required: true, message: '必填' }]} style={{ marginBottom: 8 }}>
              <Input style={{ width: 140 }} />
            </Form.Item>
            <Form.Item name="manufacturer" label="厂家" style={{ marginBottom: 8 }}>
              <Input style={{ width: 120 }} />
            </Form.Item>
            <Form.Item name="type" label="类型" rules={[{ required: true, message: '必填' }]} style={{ marginBottom: 8 }}>
              <Select options={deviceTypeOptions} style={{ width: 110 }} />
            </Form.Item>
            <Form.Item name="height_u" label="高度U" rules={[{ required: true, message: '必填' }]} style={{ marginBottom: 8 }}>
              <InputNumber min={1} max={50} style={{ width: 80 }} />
            </Form.Item>
            <Form.Item style={{ marginBottom: 8 }}>
              <Space>
                <Button type="primary" htmlType="submit">{editingModel ? '保存' : '创建'}</Button>
                {editingModel && (
                  <Button onClick={() => { setEditingModel(null); modelForm.resetFields(); }}>取消编辑</Button>
                )}
              </Space>
            </Form.Item>
          </Form>
        </div>
      </Modal>
    </div>
  );
}
