import { useEffect, useMemo, useState } from 'react';
import { Modal, Form, Select, Radio, InputNumber, Alert, App, Typography } from 'antd';
import { Device, Rack, Room } from '../../types';
import * as api from '../../tauri-api';
import { useUndo } from '../../contexts/UndoContext';

const { Text } = Typography;

const STATUS_OPTIONS = [
  { value: 'online', label: '开机' },
  { value: 'offline', label: '离线' },
  { value: 'unconfigured', label: '未上架' },
];

/** 批量编辑结果（供父级与报告展示） */
export interface BatchEditOutcome {
  updated: number;
  failures: { name: string; reason: string }[];
}

interface BatchEditModalProps {
  open: boolean;
  /** 选中的设备（批量编辑目标） */
  devices: Device[];
  /** 全量设备（自动排布计算目标机柜占用图用） */
  allDevices: Device[];
  racks: Rack[];
  rooms: Room[];
  onCancel: () => void;
  /** 提交流程结束（无论成败）后回调；父级负责 refresh */
  onDone: (outcome: BatchEditOutcome) => void;
  /** 撤销（Ctrl+Z）回填后刷新列表 */
  refresh: () => void;
}

/**
 * B2 批量编辑：多选设备统一修改 机柜 / U 位 / 状态（机房仅作机柜过滤器；
 * RackViz 数据模型中设备经机柜归属机房，不存在"只改机房"）。
 *
 * - N-10 复用：逐台调用 `updateDevice`，后端 `update_device` 校验为最终防线
 *   （U 位边界/重叠/状态白名单）；逐台独立提交，失败逐台报告（同导入"跳过+警告"语义）。
 * - U 位三模式：保持原位（搬移保留区间）/ 清空下架（rack+U 全 Clear）/ 自动排布
 *   （首适应算法：按名称升序从起始 U 起依设备固有高度安放，占用图 = 目标机柜上未被选中的设备）。
 * - N-11 撤销：提交前快照原 rack/U 位/状态，undo 逐台回填（null 传 null = Patch::Clear）。
 */
export default function BatchEditModal({ open, devices, allDevices, racks, rooms, onCancel, onDone, refresh }: BatchEditModalProps) {
  const { message, modal } = App.useApp();
  const { pushUndo } = useUndo();
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);

  const roomFilter = Form.useWatch('room_filter', form);
  const targetRackId = Form.useWatch('target_rack_id', form);
  const uMode = Form.useWatch('u_mode', form) ?? 'keep';
  const autoStartU = Form.useWatch('auto_start_u', form) ?? 1;
  const newStatus = Form.useWatch('new_status', form);

  // 打开时重置表单（避免上次编辑残留）
  useEffect(() => {
    if (open) form.resetFields();
  }, [open, form]);

  const rackOptions = useMemo(
    () => racks.filter(r => roomFilter == null || r.room_id === roomFilter),
    [racks, roomFilter],
  );

  const targetRack = targetRackId != null ? racks.find(r => r.id === Number(targetRackId)) : undefined;

  const roomLabelOf = (rack: Rack) => {
    if (rack.room_id == null) return '未分配机房';
    return rooms.find(rm => rm.id === rack.room_id)?.name ?? '未分配机房';
  };

  // 空修改拦截：机柜 / U 位 / 状态至少一项生效
  const hasChange = targetRackId != null || uMode !== 'keep' || newStatus != null;
  const autoMissingRack = uMode === 'auto' && targetRackId == null;

  // 自动排布预览（首适应）：按名称升序，从起始 U 起依设备固有高度安放；
  // 占用图 = 目标机柜上「未被选中」的已上架设备区间（选中设备会被重排，不参与占用）
  const plan = useMemo(() => {
    if (uMode !== 'auto' || targetRackId == null || !targetRack) return null;
    const height = targetRack.height_u;
    const selectedIds = new Set(devices.map(d => d.id));
    const taken: [number, number][] = allDevices
      .filter(d => d.rack_id === targetRack.id && !selectedIds.has(d.id) && d.start_u != null && d.end_u != null)
      .map(d => [d.start_u as number, d.end_u as number]);
    const entries: { id: number; name: string; start: number; end: number }[] = [];
    const failures: { name: string; reason: string }[] = [];
    const ordered = [...devices].sort((a, b) => a.name.localeCompare(b.name, 'zh-Hans-CN'));
    let cursor = autoStartU;
    for (const d of ordered) {
      const h = Math.max(1, d.height_u);
      let s = cursor;
      let placed = false;
      while (s + h - 1 <= height) {
        const e = s + h - 1;
        if (!taken.some(([ts, te]) => s <= te && ts <= e)) {
          taken.push([s, e]);
          entries.push({ id: d.id, name: d.name, start: s, end: e });
          cursor = e + 1;
          placed = true;
          break;
        }
        s += 1;
      }
      if (!placed) failures.push({ name: d.name, reason: `目标机柜空间不足（需 ${h}U）` });
    }
    return { entries, failures };
  }, [uMode, targetRackId, targetRack, devices, allDevices, autoStartU]);

  const handleApply = async () => {
    if (!hasChange) {
      message.warning('请至少选择一项要修改的内容');
      return;
    }
    if (autoMissingRack) {
      message.warning('自动排布需要先选择目标机柜');
      return;
    }
    setSubmitting(true);
    const updatedIds: number[] = [];
    const failures: { name: string; reason: string }[] = [...(plan?.failures ?? [])];
    // N-11 撤销快照：逆操作 = 回填原 rack/U 位/状态（null 值原样传 null = Patch::Clear）
    const inverses = devices.map(d => ({
      id: d.id,
      rack_id: d.rack_id,
      start_u: d.start_u,
      end_u: d.end_u,
      status: d.status,
    }));

    for (const d of devices) {
      const payload: Record<string, unknown> = {};
      if (uMode === 'clear') {
        // 下架：机柜 + U 位全清（与 DeviceDetailPanel「未上架」同语义）
        payload.rack_id = null;
        payload.start_u = null;
        payload.end_u = null;
      } else if (uMode === 'auto' && plan) {
        const p = plan.entries.find(e => e.id === d.id);
        if (!p) continue; // 排布失败已计入 failures
        payload.rack_id = targetRackId;
        payload.start_u = p.start;
        payload.end_u = p.end;
      } else if (targetRackId != null) {
        // keep：仅搬机柜，U 位键缺省 = Patch::Unset 保留原区间（后端校验边界/重叠）
        payload.rack_id = targetRackId;
      }
      if (newStatus != null) payload.status = newStatus;
      try {
        await api.updateDevice(d.id, payload);
        updatedIds.push(d.id);
      } catch (err) {
        failures.push({ name: d.name, reason: api.errorMessage(err) });
      }
    }
    setSubmitting(false);

    if (updatedIds.length > 0) {
      const doneIds = new Set(updatedIds);
      pushUndo({
        label: `批量编辑 ${updatedIds.length} 台设备`,
        undo: async () => {
          for (const inv of inverses.filter(x => doneIds.has(x.id))) {
            await api.updateDevice(inv.id, {
              rack_id: inv.rack_id,
              start_u: inv.start_u,
              end_u: inv.end_u,
              status: inv.status,
            });
          }
          refresh();
        },
      });
      if (failures.length === 0) {
        message.success(`已批量更新 ${updatedIds.length} 台设备`);
      } else {
        message.warning(`已更新 ${updatedIds.length} 台，${failures.length} 台失败（详见报告）`);
        modal.info({
          title: '批量编辑完成（部分失败）',
          width: 520,
          content: (
            <div>
              <p>成功更新 {updatedIds.length} 台，失败 {failures.length} 台：</p>
              <ul style={{ paddingLeft: 20, margin: 0 }}>
                {failures.map(f => (
                  <li key={f.name}>
                    <Text type="danger">{f.name}</Text>：{f.reason}
                  </li>
                ))}
              </ul>
            </div>
          ),
        });
      }
    } else if (failures.length > 0) {
      modal.error({
        title: '批量编辑失败',
        width: 520,
        content: (
          <ul style={{ paddingLeft: 20, margin: 0 }}>
            {failures.map(f => (
              <li key={f.name}>
                <Text type="danger">{f.name}</Text>：{f.reason}
              </li>
            ))}
          </ul>
        ),
      });
    }
    onDone({ updated: updatedIds.length, failures });
  };

  return (
    <Modal
      title={`批量编辑（已选 ${devices.length} 台）`}
      open={open}
      onCancel={onCancel}
      onOk={handleApply}
      okText={`应用到 ${devices.length} 台`}
      okButtonProps={{ disabled: !hasChange || autoMissingRack, loading: submitting }}
      cancelText="取消"
      destroyOnHidden
    >
      <Alert
        message={`以下修改将应用到全部 ${devices.length} 台选中设备`}
        description="逐台独立提交：个别设备不满足校验（如 U 位冲突）时仅该台失败，其余正常更新，可 Ctrl+Z 撤销。"
        type="info"
        showIcon
        style={{ marginBottom: 16 }}
      />
      <Form form={form} layout="vertical" initialValues={{ room_filter: null, target_rack_id: null, u_mode: 'keep', auto_start_u: 1, new_status: null }}>
        <Form.Item name="room_filter" label="机房（过滤机柜列表）">
          <Select
            allowClear
            placeholder="全部机房"
            options={rooms.map(rm => ({ value: rm.id, label: rm.name }))}
          />
        </Form.Item>
        <Form.Item name="target_rack_id" label="目标机柜">
          <Select
            allowClear
            placeholder="保持不变"
            options={rackOptions.map(r => ({
              value: r.id,
              label: `${r.name}（${roomLabelOf(r)} · ${r.height_u}U）`,
            }))}
          />
        </Form.Item>
        <Form.Item name="u_mode" label="U 位">
          <Radio.Group>
            <Radio value="keep">保持原 U 位</Radio>
            <Radio value="clear">清空 U 位（下架）</Radio>
            <Radio value="auto">自动排布</Radio>
          </Radio.Group>
        </Form.Item>
        {uMode === 'auto' && (
          <>
            <Form.Item name="auto_start_u" label="排布起始 U" rules={[{ required: true, message: '请填写起始 U' }]}>
              <InputNumber min={1} max={targetRack?.height_u ?? 100} style={{ width: 120 }} />
            </Form.Item>
            {plan && (
              <Alert
                type={plan.failures.length > 0 ? 'warning' : 'success'}
                showIcon
                message={`预览：可排布 ${plan.entries.length} 台${plan.failures.length > 0 ? `，${plan.failures.length} 台放不下` : ''}`}
                description={
                  <div style={{ maxHeight: 140, overflow: 'auto' }}>
                    {plan.entries.map(e => (
                      <div key={e.id}>U{e.start}-U{e.end} · {e.name}</div>
                    ))}
                    {plan.failures.map(f => (
                      <div key={f.name}><Text type="danger">{f.name}：{f.reason}</Text></div>
                    ))}
                  </div>
                }
                style={{ marginBottom: 16 }}
              />
            )}
          </>
        )}
        <Form.Item name="new_status" label="状态">
          <Select allowClear placeholder="保持不变" options={STATUS_OPTIONS} />
        </Form.Item>
      </Form>
    </Modal>
  );
}
