import { Tag, Space, Button } from 'antd';
import { EditOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { Device, DeviceModel, Rack, Room, DeviceSortField } from '../../types';
import HighlightText from './HighlightText';

export type DeviceColumnKey = 'name' | 'model' | 'room' | 'rack' | 'position' | 'ip' | 'serial_no' | 'asset_no' | 'department' | 'owner' | 'status' | 'created_at' | 'updated_at' | 'action';

export const ALL_COLUMNS: { key: DeviceColumnKey; title: string }[] = [
  { key: 'name', title: '设备名称' },
  { key: 'model', title: '型号' },
  { key: 'room', title: '机房' },
  { key: 'rack', title: '机柜' },
  { key: 'position', title: '位置' },
  { key: 'ip', title: 'IP' },
  { key: 'serial_no', title: '序列号' },
  { key: 'asset_no', title: '资产编号' },
  { key: 'department', title: '使用部门' },
  { key: 'owner', title: '责任人' },
  { key: 'status', title: '状态' },
  { key: 'created_at', title: '创建时间' },
  { key: 'updated_at', title: '更新时间' },
  { key: 'action', title: '操作' },
];

export const DEFAULT_COLUMN_WIDTHS: Record<string, number> = {
  name: 160,
  model: 160,
  room: 110,
  rack: 110,
  position: 90,
  ip: 140,
  serial_no: 140,
  asset_no: 130,
  department: 120,
  owner: 100,
  status: 90,
  created_at: 150,
  updated_at: 150,
  action: 120,
};

/** ISO8601(UTC) → 本地 "YYYY-MM-DD HH:mm"；非法/空值原样兜底 */
function formatTimestamp(v: string | null): string {
  if (!v) return '-';
  const d = new Date(v);
  if (Number.isNaN(d.getTime())) return v;
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

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

/** 编辑/删除/定位动作回调（由 DeviceList 提供，避免模块依赖 modal 实例） */
export interface DeviceActions {
  onEdit: (device: Device) => void;
  onDelete: (device: Device) => void;
  /** B1 全局搜索：跳转机柜视图并高亮该设备（未上架设备由 UI 禁用入口） */
  onLocate: (device: Device) => void;
}

/** 服务端排序白名单（与后端 `query_devices` 白名单一一对应；isSortField 守卫的数据源） */
const SORT_FIELD_SET: ReadonlySet<string> = new Set([
  'name', 'ip_addresses', 'serial_no', 'asset_no', 'status', 'power_watt', 'created_at', 'updated_at', 'rack_id',
]);

/** AntD sorter field → DeviceSortField 类型守卫（C2a：替代裸 `as DeviceSortField` 收窄） */
export function isSortField(v: unknown): v is DeviceSortField {
  return typeof v === 'string' && SORT_FIELD_SET.has(v);
}

/** 服务端分页/排序/高亮所需的受控状态（由 DeviceList 传入） */
export interface DeviceColumnOptions {
  /** 当前搜索关键词（用于高亮；空则不包裹 <mark>） */
  search?: string | null;
  /** 当前服务端排序字段（C1：DeviceQuery.sort_field 生成即 string；白名单由发送侧 UI 约束） */
  sortField?: string | null;
  /** 当前服务端排序方向（传输层为 string；serverSorter 内已做 asc/desc 判定） */
  sortOrder?: string | null;
}

/**
 * 生成「服务端排序」列配置（受控）：
 * 仅白名单字段可排序（name|ip_addresses|serial_no|asset_no|status|power_watt|created_at|updated_at|rack_id），
 * 其余列不给 sorter，避免前端裸排序与后端白名单不一致。
 */
function serverSorter(field: DeviceSortField, opts: DeviceColumnOptions) {
  const active = opts.sortField === field && (opts.sortOrder === 'asc' || opts.sortOrder === 'desc');
  return {
    sorter: true as const,
    sortOrder: active ? (opts.sortOrder === 'desc' ? ('descend' as const) : ('ascend' as const)) : null,
  };
}

/** 设备台账表格列定义（外置模块：避免每次渲染重建 + 缩小 DeviceList） */
export function buildDeviceColumns(
  models: DeviceModel[],
  racks: Rack[],
  rooms: Room[],
  actions: DeviceActions,
  opts: DeviceColumnOptions = {},
): ColumnsType<Device> {
  const keyword = opts.search ?? '';

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
    {
      title: '设备名称', dataIndex: 'name', key: 'name',
      ...serverSorter('name', opts),
      render: (v: string) => <HighlightText text={v} keyword={keyword} />,
    },
    { title: '型号', dataIndex: 'device_model_id', key: 'model', render: (id: number | null) => getModelName(id) },
    { title: '机房', dataIndex: 'rack_id', key: 'room', render: (id: number | null) => getRoomName(id) },
    {
      title: '机柜', dataIndex: 'rack_id', key: 'rack',
      ...serverSorter('rack_id', opts),
      render: (id: number | null) => getRackName(id),
    },
    { title: '位置', key: 'position', render: (_: unknown, record: Device) => record.start_u && record.end_u ? `${record.start_u}-${record.end_u}U` : '-' },
    {
      title: 'IP', dataIndex: 'ip_addresses', key: 'ip',
      ...serverSorter('ip_addresses', opts),
      render: (ips: string) => <HighlightText text={ips} keyword={keyword} />,
    },
    {
      title: '序列号', dataIndex: 'serial_no', key: 'serial_no',
      ...serverSorter('serial_no', opts),
      render: (v: string) => <HighlightText text={v} keyword={keyword} />,
    },
    {
      title: '资产编号', dataIndex: 'asset_no', key: 'asset_no',
      ...serverSorter('asset_no', opts),
      render: (v: string) => <HighlightText text={v} keyword={keyword} />,
    },
    { title: '使用部门', dataIndex: 'department', key: 'department', render: (dept: string) => dept || '-' },
    { title: '责任人', dataIndex: 'owner', key: 'owner', render: (owner: string) => owner || '-' },
    {
      title: '状态', dataIndex: 'status', key: 'status',
      ...serverSorter('status', opts),
      render: (status: string) => getStatusTag(status),
    },
    {
      title: '创建时间', dataIndex: 'created_at', key: 'created_at', width: 150,
      ...serverSorter('created_at', opts),
      render: (v: string | null) => formatTimestamp(v),
    },
    {
      title: '更新时间', dataIndex: 'updated_at', key: 'updated_at', width: 150,
      ...serverSorter('updated_at', opts),
      render: (v: string | null) => formatTimestamp(v),
    },
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
