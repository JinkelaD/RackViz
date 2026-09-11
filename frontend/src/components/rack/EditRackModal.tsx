import { Modal, Form, Input, Button, Select } from 'antd';
import type { Rack, Room } from '../../types';

interface EditRackModalProps {
  rack: Rack | null;
  visible: boolean;
  rooms: Room[];
  onClose: () => void;
  onSave: (id: number, values: { name: string; room_id: number | null }) => Promise<void>;
}

/** 机柜页：双击机柜头部弹出的编辑表单 */
export default function EditRackModal({ rack, visible, rooms, onClose, onSave }: EditRackModalProps) {
  if (!rack) return null;

  return (
    <Modal
      title={null}
      open={visible}
      onCancel={onClose}
      footer={null}
      width={420}
      destroyOnHidden
      className="device-edit-modal"
    >
      <div className="device-edit-wrap">
        <div className="device-edit-header">
          <div className="device-edit-icon">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" strokeWidth="1.5">
              <rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="8" x2="22" y2="8"/><line x1="2" y1="16" x2="22" y2="16"/><line x1="8" y1="2" x2="8" y2="22"/>
            </svg>
          </div>
          <div className="device-edit-title">编辑机柜</div>
          <div className="device-edit-subtitle">{rack.name} · {rack.height_u}U</div>
        </div>
        <Form
          layout="vertical"
          size="small"
          initialValues={{
            name: rack.name,
            room_id: rack.room_id || undefined,
          }}
          onFinish={async (values: { name: string; room_id?: number | null }) => {
            await onSave(rack.id, { name: values.name.trim(), room_id: values.room_id ?? null });
            onClose();
          }}
        >
          <div className="device-edit-body">
            <Form.Item name="name" label="机柜名称" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
            <Form.Item name="room_id" label="所属机房">
              <Select placeholder="选择机房" allowClear>
                {rooms.map(r => (
                  <Select.Option key={r.id} value={r.id}>{r.name}</Select.Option>
                ))}
              </Select>
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
