import { useEffect, useState } from 'react';
import { Modal, Form, Input, Select, InputNumber } from 'antd';
import type { Device, DeviceModel, Rack } from '../../types';
import { DEVICE_TYPE_LABELS } from '../../constants/labels';

/** N-10：单个 IPv4 地址校验 */
const IPV4_RE = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/;
const DEVICE_STATUSES = ['online', 'offline', 'unconfigured'] as const;

/** N-10：逗号/分号/空白分隔的 IP 列表校验；允许空；每项须为 IPv4 或含冒号（放行 IPv6） */
function isValidIpList(raw: string): boolean {
  const parts = raw.split(/[,，;；\s]+/).map(s => s.trim()).filter(Boolean);
  if (parts.length === 0) return true;
  return parts.every(p => IPV4_RE.test(p) || p.includes(':'));
}

/** 未选机柜时的宽松 U 位上限（选中机柜后按其真实高度动态收紧；后端为最终防线） */
const U_MAX_FALLBACK = 100;

interface DeviceFormModalProps {
  open: boolean;
  /** null = 新建 */
  editing: Device | null;
  models: DeviceModel[];
  racks: Rack[];
  onCancel: () => void;
  /** 由父组件负责 create/update、错误提示与关闭 */
  onSave: (payload: Record<string, unknown>, editing: Device | null) => Promise<void>;
}

/** 设备台账页：新增 / 编辑设备表单弹窗 */
export default function DeviceFormModal({ open, editing, models, racks, onCancel, onSave }: DeviceFormModalProps) {
  const [form] = Form.useForm();
  const [saving, setSaving] = useState(false);

  // N-10：U 位上限按所选机柜动态收紧（未选机柜时宽松上限；后端按实际机柜做最终校验）
  const rackIdWatch = Form.useWatch('rack_id', form);
  const selectedRack = racks.find(r => r.id === Number(rackIdWatch));
  const uMax = selectedRack?.height_u ?? U_MAX_FALLBACK;

  // 打开时按编辑/新增填充
  useEffect(() => {
    if (!open) return;
    if (editing) {
      form.setFieldsValue({
        ...editing,
        device_model_id: editing.device_model_id,
        rack_id: editing.rack_id,
      });
    } else {
      form.resetFields();
    }
  }, [open, editing, form]);

  /** 选择型号 → 依据型号高度自动推算 end_u */
  const handleModelChange = (modelId: number) => {
    const model = models.find(m => m.id === modelId);
    if (model) {
      form.setFieldsValue({ end_u: (form.getFieldValue('start_u') || 1) + model.height_u - 1 });
    }
  };

  /** 修改起始 U 位 → 联动 end_u */
  const handleStartUChange = (startU: number | null) => {
    const modelId = form.getFieldValue('device_model_id');
    const model = models.find(m => m.id === Number(modelId));
    if (model && startU) {
      form.setFieldsValue({ end_u: startU + model.height_u - 1 });
    }
  };

  /** 切换机柜 → U 位上限变化，待新 rules 渲染生效后重校验已填 U 位（超界即时提示）。
   *  React 19 concurrent 下 useWatch 驱动的重渲染在事件处理后调度，
   *  同步 validateFields 会用旧 rules（max 仍为旧机柜高度），须延后到渲染完成。 */
  const handleRackChange = () => {
    setTimeout(() => {
      form.validateFields(['start_u', 'end_u']).catch(() => {
        // 校验失败由字段级 message 展示，此处仅吞掉 reject
      });
    }, 50);
  };

  const handleOk = async () => {
    try {
      const values = await form.validateFields();
      const payload: Record<string, unknown> = {
        ...values,
        rack_id: values.status === 'unconfigured' ? null : (values.rack_id ?? null),
        start_u: values.status === 'unconfigured' ? null : values.start_u,
        end_u: values.status === 'unconfigured' ? null : values.end_u,
      };
      setSaving(true);
      await onSave(payload, editing);
    } catch (error) {
      // 校验失败或保存失败：antd 已在字段上提示；父组件负责 message 与关闭
      if (error instanceof Error) {
        console.error('保存设备失败:', error);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      title={editing ? '编辑设备' : '添加设备'}
      open={open}
      onOk={handleOk}
      onCancel={onCancel}
      confirmLoading={saving}
      width={600}
    >
      <Form form={form} layout="vertical">
        <Form.Item
          name="name"
          label="设备名称"
          rules={[
            { required: true, whitespace: true, message: '请输入设备名称' },
            { max: 100, message: '设备名称不超过 100 个字符' },
          ]}
        >
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
          <Select placeholder="请选择机柜（可选）" allowClear onChange={handleRackChange}>
            {racks.map(rack => (
              <Select.Option key={rack.id} value={rack.id}>
                {rack.name} ({rack.height_u}U)
              </Select.Option>
            ))}
          </Select>
        </Form.Item>
        <Form.Item
          name="status"
          label="设备状态"
          rules={[
            {
              validator: (_rule, value) =>
                !value || (DEVICE_STATUSES as readonly string[]).includes(value)
                  ? Promise.resolve()
                  : Promise.reject(new Error('设备状态取值非法')),
            },
          ]}
        >
          <Select placeholder="请选择状态">
            <Select.Option value="online">开机</Select.Option>
            <Select.Option value="offline">离线</Select.Option>
            <Select.Option value="unconfigured">未上架</Select.Option>
          </Select>
        </Form.Item>
        <Form.Item
          name="start_u"
          label="起始U位"
          rules={[{ type: 'number', min: 1, max: uMax, message: `起始U位须在 1~${uMax} 之间` }]}
        >
          <InputNumber min={1} max={uMax} placeholder="起始U位" onChange={handleStartUChange} />
        </Form.Item>
        <Form.Item
          name="end_u"
          label="结束U位"
          dependencies={['start_u']}
          rules={[
            { type: 'number', min: 1, max: uMax, message: `结束U位须在 1~${uMax} 之间` },
            ({ getFieldValue }) => ({
              validator(_rule, value) {
                const start = getFieldValue('start_u');
                if (value == null || start == null) return Promise.resolve();
                if (value < start) return Promise.reject(new Error('结束U位不能小于起始U位'));
                return Promise.resolve();
              },
            }),
          ]}
        >
          <InputNumber min={1} max={uMax} placeholder="结束U位" disabled />
        </Form.Item>
        <Form.Item
          name="ip_addresses"
          label="IP地址"
          rules={[
            {
              validator: (_rule, value) =>
                isValidIpList(String(value ?? ''))
                  ? Promise.resolve()
                  : Promise.reject(new Error('IP 地址格式不正确（多个请用逗号分隔）')),
            },
          ]}
        >
          <Input placeholder="多个IP用逗号分隔" />
        </Form.Item>
        <Form.Item
          name="serial_no"
          label="序列号"
          rules={[{ max: 100, message: '序列号不超过 100 个字符' }]}
        >
          <Input placeholder="设备序列号" />
        </Form.Item>
        <Form.Item
          name="asset_no"
          label="资产编号"
          rules={[{ max: 100, message: '资产编号不超过 100 个字符' }]}
        >
          <Input placeholder="资产编号（便于查账）" />
        </Form.Item>
        <Form.Item
          name="department"
          label="使用部门"
          rules={[{ max: 100, message: '使用部门不超过 100 个字符' }]}
        >
          <Input placeholder="所属部门" />
        </Form.Item>
        <Form.Item
          name="owner"
          label="责任人"
          rules={[{ max: 50, message: '责任人不超过 50 个字符' }]}
        >
          <Input placeholder="设备负责人" />
        </Form.Item>
        <Form.Item
          name="function"
          label="功能描述"
          rules={[{ max: 200, message: '功能描述不超过 200 个字符' }]}
        >
          <Input placeholder="设备功能描述" />
        </Form.Item>
        <Form.Item name="purchase_date" label="采购日期">
          <Input type="date" />
        </Form.Item>
        <Form.Item
          name="warranty_expire"
          label="质保到期"
          dependencies={['purchase_date']}
          rules={[
            ({ getFieldValue }) => ({
              validator(_rule, value) {
                const purchase = getFieldValue('purchase_date');
                if (!value || !purchase) return Promise.resolve();
                if (value < purchase) return Promise.reject(new Error('质保到期不能早于采购日期'));
                return Promise.resolve();
              },
            }),
          ]}
        >
          <Input type="date" />
        </Form.Item>
      </Form>
    </Modal>
  );
}
