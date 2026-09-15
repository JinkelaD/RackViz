import { useCallback, useEffect, useState } from 'react';
import { Alert, App, Button, Checkbox, Modal, Progress, Radio, Space, Typography } from 'antd';
import { FolderOpenOutlined } from '@ant-design/icons';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type { ImportOptions, ImportResult } from '../../types';
import * as api from '../../tauri-api';
import { useImportProgress } from '../../hooks/useImportProgress';

const { Text, Paragraph } = Typography;

interface ImportWizardModalProps {
  open: boolean;
  onClose: () => void;
  /** 导入成功后通知父级刷新列表 */
  onImported: () => void;
}

/** 各阶段文案（N-06 进度提示） */
const PHASE_TEXT: Record<string, string> = {
  parsing: '正在解析文件…',
  importing: '正在导入数据…',
  done: '导入完成',
};

/**
 * Excel 导入向导（N-04/N-05/N-06/N-21）。
 * - 选择文件 + 更新模式（跳过/覆盖）+ 自动关联机房
 * - 进度条消费 `import://progress` 事件（结束时在 finally 中 unlisten）
 * - 完成后展示 新增/更新/跳过 + 新建型号数 + 告警项
 */
export default function ImportWizardModal({ open, onClose, onImported }: ImportWizardModalProps) {
  const { message } = App.useApp();
  const { progress, begin, end } = useImportProgress();

  const [filePath, setFilePath] = useState<string | null>(null);
  const [updateMode, setUpdateMode] = useState<'skip' | 'overwrite'>('skip');
  const [linkRoom, setLinkRoom] = useState(true);
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);

  // 每次打开重置向导状态
  useEffect(() => {
    if (open) {
      setFilePath(null);
      setUpdateMode('skip');
      setLinkRoom(true);
      setImporting(false);
      setResult(null);
    }
  }, [open]);

  const handlePickFile = useCallback(async () => {
    try {
      const selected = await openDialog({
        filters: [{ name: 'Excel 文件', extensions: ['xlsx', 'xls'] }],
        multiple: false,
      });
      if (!selected) return;
      const path = typeof selected === 'string' ? selected : String((selected as { path?: string }).path ?? '');
      setFilePath(path);
      setResult(null);
    } catch (err) {
      message.error(`选择文件失败：${api.errorMessage(err)}`);
    }
  }, [message]);

  const handleImport = useCallback(async () => {
    if (!filePath) {
      message.warning('请先选择要导入的 Excel 文件');
      return;
    }
    const options: ImportOptions = { update_mode: updateMode, link_room: linkRoom };
    setImporting(true);
    setResult(null);
    await begin();
    try {
      const r = await api.importExcelFromPath(filePath, options);
      setResult(r);
      onImported();
    } catch (err) {
      message.error(`导入失败：${api.errorMessage(err)}`);
    } finally {
      // 必须在 finally 中 unlisten（§8-8）
      end();
      setImporting(false);
    }
  }, [filePath, updateMode, linkRoom, begin, end, onImported, message]);

  const percent = (() => {
    if (!progress) return 0;
    if (progress.total > 0) return Math.min(100, Math.round((progress.processed / progress.total) * 100));
    return progress.phase === 'done' ? 100 : 0;
  })();

  const footer = result ? (
    <Button type="primary" onClick={onClose}>完成</Button>
  ) : (
    <Space>
      <Button onClick={onClose} disabled={importing}>取消</Button>
      <Button type="primary" loading={importing} disabled={!filePath || importing} onClick={handleImport}>
        开始导入
      </Button>
    </Space>
  );

  return (
    <Modal
      open={open}
      title="Excel 导入向导"
      width={560}
      onCancel={importing ? undefined : onClose}
      maskClosable={!importing}
      footer={footer}
    >
      <Space direction="vertical" size="middle" style={{ width: '100%' }}>
        <div>
          <Text strong>1. 选择文件</Text>
          <div className="import-wizard-file">
            <Button icon={<FolderOpenOutlined />} onClick={handlePickFile} disabled={importing}>
              选择 Excel 文件
            </Button>
            <Text className="import-wizard-path" ellipsis={{ tooltip: filePath ?? '' }}>
              {filePath ?? '未选择文件'}
            </Text>
          </div>
        </div>

        <div>
          <Text strong>2. 更新模式</Text>
          <div style={{ marginTop: 8 }}>
            <Radio.Group value={updateMode} onChange={(e) => setUpdateMode(e.target.value)} disabled={importing}>
              <Radio value="skip">跳过已存在（不修改已有记录）</Radio>
              <Radio value="overwrite">覆盖已存在（按查重键更新字段）</Radio>
            </Radio.Group>
          </div>
        </div>

        <div>
          <Checkbox checked={linkRoom} onChange={(e) => setLinkRoom(e.target.checked)} disabled={importing}>
            自动关联机房（依据文件中的机房列，幂等创建）
          </Checkbox>
        </div>

        <Alert
          type="info"
          showIcon
          message="设备类型列说明"
          description={
            <Paragraph style={{ margin: 0, fontSize: 12 }}>
              第 16 列「设备类型」可填中文名（如 服务器 / 交换机 / 路由器 / 存储阵列 / NAS 存储 / 网安设备 / 负载均衡 / 其他）
              或英文枚举（server / switch / router / storage / nas / security / loadbalancer / other）。
              未知值或空值将兜底为「其他」并在完成后提示，不影响导入。旧文件（无此列）仍可正常导入。
            </Paragraph>
          }
        />

        {(importing || progress) && (
          <div>
            <Progress percent={percent} status={importing ? 'active' : 'normal'} />
            <Text type="secondary" style={{ fontSize: 12 }}>
              {progress ? PHASE_TEXT[progress.phase] ?? '' : ''}
              {progress && progress.total > 0 ? `（${progress.processed} / ${progress.total}）` : ''}
            </Text>
          </div>
        )}

        {result && (
          <Alert
            type={result.errors.length > 0 ? 'warning' : 'success'}
            showIcon
            message="导入完成"
            description={
              <div className="import-summary">
                <div>
                  新增 <b>{result.imported}</b> 条 · 更新 <b>{result.updated}</b> 条 · 跳过 <b>{result.skipped}</b> 条 · 合计 <b>{result.total}</b> 条
                </div>
                <div>新建型号 <b>{result.models_created}</b> 个</div>
                {result.warnings.length > 0 && (
                  <div className="import-summary-warnings">
                    <div>告警（{result.warnings.length}）：</div>
                    <ul>
                      {result.warnings.slice(0, 20).map((w, i) => <li key={i}>{w}</li>)}
                      {result.warnings.length > 20 && <li>… 其余 {result.warnings.length - 20} 条略</li>}
                    </ul>
                  </div>
                )}
                {result.errors.length > 0 && (
                  <div className="import-summary-errors">
                    <div>失败（{result.errors.length}）：</div>
                    <ul>
                      {result.errors.slice(0, 20).map((e, i) => <li key={i}>{e}</li>)}
                    </ul>
                  </div>
                )}
              </div>
            }
          />
        )}
      </Space>
    </Modal>
  );
}
