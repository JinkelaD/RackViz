import { useCallback, useEffect, useState } from 'react';
import { App, Button, Modal, Table, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { BackupInfo } from '../../types';
import * as api from '../../tauri-api';

/** 字节数 → 可读大小 */
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

/** UTC ISO8601 → 本地 "YYYY-MM-DD HH:mm:ss" */
function formatTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso || '-';
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

interface BackupManagerModalProps {
  open: boolean;
  onClose: () => void;
  /** 恢复排期成功后回调（提示重启） */
  onRestoreScheduled?: (message: string) => void;
}

/**
 * A2：自动备份管理面板。
 *
 * 列出 `<数据目录>/backups/` 下的自动备份（最新在前，含有效性校验），
 * 支持从备份恢复（复用 N-18 延迟交换流程，重启生效）与删除（名称白名单校验）。
 */
export default function BackupManagerModal({ open, onClose, onRestoreScheduled }: BackupManagerModalProps) {
  const { modal, message } = App.useApp();
  const [items, setItems] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [restoring, setRestoring] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.listBackups();
      setItems(list);
    } catch (err) {
      message.error(`加载备份列表失败：${api.errorMessage(err)}`);
    } finally {
      setLoading(false);
    }
  }, [message]);

  useEffect(() => {
    if (open) {
      void load();
    }
  }, [open, load]);

  const handleRestore = useCallback((b: BackupInfo) => {
    if (!b.valid) {
      message.warning('该备份未通过完整性校验，不可用于恢复');
      return;
    }
    modal.confirm({
      title: '确认从备份恢复',
      content: (
        <div>
          <p>将从「{b.name}」恢复，<b>覆盖当前全部数据</b>，此操作不可撤销。</p>
          <p>恢复后需要<b>重启应用</b>才能生效；恢复前会自动备份当前数据。</p>
        </div>
      ),
      okText: '恢复',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        setRestoring(b.name);
        try {
          const res = await api.restoreDatabase(b.full_path);
          onRestoreScheduled?.(res.message);
          onClose();
        } catch (err) {
          message.error(`恢复失败：${api.errorMessage(err)}`);
        } finally {
          setRestoring(null);
        }
      },
    });
  }, [modal, message, onRestoreScheduled, onClose]);

  const handleDelete = useCallback((b: BackupInfo) => {
    modal.confirm({
      title: '确认删除备份',
      content: `确定删除「${b.name}」吗？删除后不可恢复。`,
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        setDeleting(b.name);
        try {
          await api.deleteBackup(b.name);
          message.success('备份已删除');
          await load();
        } catch (err) {
          message.error(`删除失败：${api.errorMessage(err)}`);
        } finally {
          setDeleting(null);
        }
      },
    });
  }, [modal, message, load]);

  const columns: ColumnsType<BackupInfo> = [
    { title: '文件名', dataIndex: 'name', key: 'name', ellipsis: true },
    { title: '大小', dataIndex: 'size_bytes', key: 'size', width: 90, render: (v: number) => formatSize(v) },
    { title: '备份时间', dataIndex: 'modified_at', key: 'modified_at', width: 170, render: (v: string) => formatTime(v) },
    {
      title: '状态',
      dataIndex: 'valid',
      key: 'valid',
      width: 90,
      render: (v: boolean) => (v ? <Tag color="success">可用</Tag> : <Tag color="error">损坏</Tag>),
    },
    {
      title: '操作',
      key: 'action',
      width: 140,
      render: (_: unknown, b: BackupInfo) => (
        <Button.Group size="small">
          <Button type="link" size="small" disabled={!b.valid} loading={restoring === b.name} onClick={() => handleRestore(b)}>
            恢复
          </Button>
          <Button type="link" size="small" danger loading={deleting === b.name} onClick={() => handleDelete(b)}>
            删除
          </Button>
        </Button.Group>
      ),
    },
  ];

  return (
    <Modal
      open={open}
      onCancel={onClose}
      width={680}
      title="自动备份管理"
      footer={<Button onClick={onClose}>关闭</Button>}
    >
      <p className="batch-print-hint">
        应用每日首次启动会自动备份当前数据（滚动保留最近 7 份）；从此处可将数据恢复到任一备份点，恢复后需重启应用。
      </p>
      <Table
        rowKey="name"
        size="small"
        loading={loading}
        dataSource={items}
        columns={columns}
        pagination={false}
        locale={{ emptyText: '暂无自动备份（将在下次启动时生成）' }}
      />
    </Modal>
  );
}
