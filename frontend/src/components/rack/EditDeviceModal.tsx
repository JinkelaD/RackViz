import { Modal, Form, Input, Button, Select } from 'antd';
import type { Device, DeviceModel, Rack } from '../../types';

interface EditDeviceModalProps {
  device: Device | null;
  visible: boolean;
  models: DeviceModel[];
  racks: Rack[];
  onClose: () => void;
  /** values 为表单原始值（snake_case），由调用方做更新 */
  onSave: (id: number, values: Record<string, unknown>) => Promise<void>;
}

/** 机柜页：双击设备弹出的编辑表单 */
export default function EditDeviceModal({ device, visible, models, racks, onClose, onSave }: EditDeviceModalProps) {
  if (!device) return null;
  const rack = racks.find(r => r.id === device.rack_id);

  return (
    <Modal
      title={null}
      open={visible}
      onCancel={onClose}
      footer={null}
      width={440}
      destroyOnHidden
      className="device-edit-modal"
    >
      <div className="device-edit-wrap">
        <div className="device-edit-header">
          <div className="device-edit-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" strokeWidth="1.5">
              <rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="6" x2="22" y2="6"/><line x1="2" y1="10" x2="22" y2="10"/><line x1="2" y1="14" x2="22" y2="14"/><line x1="2" y1="18" x2="22" y2="18"/>
            </svg>
          </div>
          <div className="device-edit-title">编辑设备</div>
          <div className="device-edit-subtitle">
            {rack?.name || '未分配'}
            {device.start_u != null ? ` · U${device.start_u}-${device.end_u}` : ''}
          </div>
        </div>
        <Form
          layout="vertical"
          size="small"
          initialValues={{
            name: device.name,
            device_model_id: device.device_model_id,
            serial_no: device.serial_no || '',
            asset_no: device.asset_no || '',
            ip_addresses: device.ip_addresses || '',
            department: device.department || '',
            owner: device.owner || '',
            purchase_date: device.purchase_date || '',
          }}
          onFinish={async (values) => {
            await onSave(device.id, values);
            onClose();
          }}
        >
          <div className="device-edit-body">
            <Form.Item name="name" label="设备名称" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
            <Form.Item name="device_model_id" label="设备型号">
              <Select placeholder="选择型号" allowClear>
                {models.map(m => (
                  <Select.Option key={m.id} value={m.id}>{m.name} ({m.height_u}U)</Select.Option>
                ))}
              </Select>
            </Form.Item>
            <Form.Item name="ip_addresses" label="IP地址">
              <Input placeholder="多个IP用逗号分隔" />
            </Form.Item>
            <Form.Item name="serial_no" label="序列号">
              <Input />
            </Form.Item>
            <Form.Item name="asset_no" label="资产编号">
              <Input />
            </Form.Item>
            <Form.Item name="department" label="使用部门">
              <Input />
            </Form.Item>
            <Form.Item name="owner" label="责任人">
              <Input />
            </Form.Item>
            <Form.Item name="purchase_date" label="采购日期">
              <Input type="date" />
            </Form.Item>
          </div>
          <div className="device-edit-footer">
            <Button onClick={onClose}>
              取消
            </Button>
            <Button type="primary" htmlType="submit">
              保存修改
            </Button>
          </div>
        </Form>
      </div>
    </Modal>
  );
}
