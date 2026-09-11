import { Tag, Space, Button } from 'antd';
import { EditOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { Device, DeviceModel, Rack, Room } from '../../types';

export type DeviceColumnKey = 'name' | 'model' | 'rack' | 'room' | 'position' | 'ip' | 'asset_no' | 'department' | 'owner' | 'status' | 'action';

export const ALL_COLUMNS: { key: DeviceColumnKey; title: string }[] = [
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

export const DEFAULT_COLUMN_WIDTHS: Record<string, number> = {
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

function getStatusTag(status: string) {
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
}

/** 编辑/删除动作回调（由 DeviceList 提供，避免模块依赖 modal 实例） */
export interface DeviceActions {
  onEdit: (device: Device) => void;
  onDelete: (device: Device) => void;
}

/** 设备台账表格列定义（外置模块：避免每次渲染重建 + 缩小 DeviceList） */
export function buildDeviceColumns(
  models: DeviceModel[],
  racks: Rack[],
  rooms: Room[],
  actions: DeviceActions,
): ColumnsType<Device> {
  const getModelName = (modelId: number | null) => {
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

  return [
    { title: '设备名称', dataIndex: 'name', key: 'name', sorter: (a, b) => a.name.localeCompare(b.name, 'zh') },
    { title: '型号', dataIndex: 'device_model_id', key: 'model', render: (id: number | null) => getModelName(id), sorter: (a, b) => getModelName(a.device_model_id).localeCompare(getModelName(b.device_model_id), 'zh') },
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
          <Button size="small" icon={<EditOutlined />} onClick={() => actions.onEdit(record)}>编辑</Button>
          <Button size="small" danger onClick={() => actions.onDelete(record)}>删除</Button>
        </Space>
      ),
    },
  ];
}
