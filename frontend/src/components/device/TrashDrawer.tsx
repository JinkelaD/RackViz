import { useCallback, useEffect, useState } from 'react';
import { App, Button, Drawer, Empty, Table, Tooltip } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { DeleteOutlined, ReloadOutlined } from '@ant-design/icons';
import type { Device } from '../../types';
import * as api from '../../tauri-api';

/** N-09：软删除后可在 N 天内恢复；超期仅标记不可恢复（不物理清理） */
const RECOVERY_WINDOW_DAYS = 30;
const MS_PER_DAY = 24 * 60 * 60 * 1000;

/** 计算剩余可恢复天数；无删除时间返回 null */
function remainingDays(deletedAt: string | null): number | null {
  if (!deletedAt) return null;
  const t = Date.parse(deletedAt);
  if (Number.isNaN(t)) return null;
  const elapsedDays = Math.floor((Date.now() - t) / MS_PER_DAY);
  return RECOVERY_WINDOW_DAYS - elapsedDays;
}

function formatTime(v: string | null): string {
  if (!v) return '-';
  const d = new Date(v);
  if (Number.isNaN(d.getTime())) return v;
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

interface TrashDrawerProps {
  open: boolean;
  onClose: () => void;
  /** 恢复成功后通知父级刷新主列表 */
  onRestored: () => void;
}

/**
 * 回收站抽屉（N-09）：列出已软删除设备、显示剩余可恢复天数、支持单条恢复。
 * 恢复冲突（序列号/资产编号被占用、超 30 天）时原样提示后端错误文案。
 */
export default function TrashDrawer({ open, onClose, onRestored }: TrashDrawerProps) {
  const { message, modal } = App.useApp();
  const [devices, setDevices] = useState<Device[]>([]);
  const [loading, setLoading] = useState(false);
  const [restoringId, setRestoringId] = useState<number | null>(null);
  const [purgingId, setPurgingId] = useState<number | null>(null);

  const load = useCallback(() => {
    setLoading(true);
    api.listDeletedDevices()
      .then(setDevices)
      .catch((err) => message.error(`加载回收站失败：${api.errorMessage(err)}`))
      .finally(() => setLoading(false));
  }, [message]);

  useEffect(() => {
    if (open) load();
  }, [open, load]);

  const handleRestore = useCallback(async (device: Device) => {
    setRestoringId(device.id);
    try {
      await api.restoreDevice(device.id); // lint-ok：try/catch 包裹（下方 catch）
      message.success(`已恢复设备「${device.name}」`);
      load();
      onRestored();
    } catch (err) {
      // 冲突/超期：原样透传后端错误文案（如「序列号已被设备「Y」占用」）
      message.error(api.errorMessage(err));
    } finally {
      setRestoringId(null);
    }
  }, [message, load, onRestored]);

  // 彻底删除：物理清除、跳过 30 天保留期、不可恢复。防误操作强制**两次确认**：
  // ① 设备级确认（点明设备名与后果）→ ② 最终警告（红色危险按钮）后才执行。
  const handlePurge = useCallback((device: Device) => {
    modal.confirm({
      title: `确认彻底删除「${device.name}」？`,
      content: '该设备将从数据库中物理清除，跳过 30 天保留期，删除后无法恢复。',
      okText: '继续删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: () => {
        return new Promise<void>((resolve) => {
          modal.confirm({
            title: '最终确认：永久删除',
            content: `「${device.name}」的数据将被永久清除且无法撤销。确定继续吗？`,
            okText: '永久删除',
            okType: 'danger',
            cancelText: '再想想',
            onOk: async () => {
              setPurgingId(device.id);
              try {
                await api.purgeDevice(device.id);
                message.success(`已彻底删除设备「${device.name}」`);
                load();
                onRestored();
              } catch (err) {
                message.error(`彻底删除失败：${api.errorMessage(err)}`);
              } finally {
                setPurgingId(null);
              }
            },
            afterClose: resolve,
          });
        });
      },
    });
  }, [modal, message, load, onRestored]);

  const columns: ColumnsType<Device> = [
    { title: '设备名称', dataIndex: 'name', key: 'name', render: (v: string) => v || '-', ellipsis: true },
    { title: '序列号', dataIndex: 'serial_no', key: 'serial_no', render: (v: string) => v || '-', ellipsis: true },
    { title: '资产编号', dataIndex: 'asset_no', key: 'asset_no', render: (v: string) => v || '-', ellipsis: true },
    {
      title: '删除时间',
      dataIndex: 'deleted_at',
      key: 'deleted_at',
      render: (v: string | null) => formatTime(v),
    },
    {
      title: '剩余可恢复',
      key: 'remaining',
      render: (_: unknown, record: Device) => {
        const days = remainingDays(record.deleted_at);
        if (days === null) return <span className="trash-days-expired">-</span>;
        if (days <= 0) return <span className="trash-days-expired">已超期不可恢复</span>;
        if (days <= 7) return <span className="trash-days-warning">{days} 天</span>;
        return <span className="trash-days-normal">{days} 天</span>;
      },
    },
    {
      title: '操作',
      key: 'action',
      width: 170,
      render: (_: unknown, record: Device) => {
        const days = remainingDays(record.deleted_at);
        const expired = days !== null && days <= 0;
        const busy = restoringId !== null || purgingId !== null;
        const restoreBtn = (
          <Button
            size="small"
            loading={restoringId === record.id}
            disabled={expired || busy}
            onClick={() => handleRestore(record)}
          >
            恢复
          </Button>
        );
        return (
          <span style={{ display: 'inline-flex', gap: 6 }}>
            {expired
              ? <Tooltip title="已超过 30 天恢复期，不可恢复">{restoreBtn}</Tooltip>
              : restoreBtn}
            <Button
              size="small"
              danger
              icon={<DeleteOutlined />}
              loading={purgingId === record.id}
              disabled={busy}
              onClick={() => handlePurge(record)}
            >
              彻底删除
            </Button>
          </span>
        );
      },
    },
  ];

  return (
    <Drawer
      title="回收站（30 天内可恢复）"
      width={720}
      open={open}
      onClose={onClose}
      styles={{ body: { padding: '16px' } }}
      extra={<Button icon={<ReloadOutlined />} onClick={load} loading={loading}>刷新</Button>}
    >
      <Table<Device>
        rowKey="id"
        size="small"
        dataSource={devices}
        columns={columns}
        loading={loading}
        pagination={{ pageSize: 20, showTotal: (total: number) => `共 ${total} 台` }}
        locale={{ emptyText: <Empty description="回收站为空" /> }}
        scroll={{ x: 'max-content' }}
      />
    </Drawer>
  );
}
