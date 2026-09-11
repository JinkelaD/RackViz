import { useState, useEffect, useMemo } from 'react';
import { Modal, Table, Button, Input, Form, Select, InputNumber, Space, Tag } from 'antd';
import type { DeviceModel } from '../../types';
import {
  DEVICE_TYPE_LABELS,
  DEVICE_TYPE_OPTIONS,
  DEVICE_TYPE_TAG_COLORS,
} from '../../constants/labels';

function getTypeTag(type: string) {
  return <Tag color={DEVICE_TYPE_TAG_COLORS[type]}>{DEVICE_TYPE_LABELS[type] || type}</Tag>;
}

interface ModelManageModalProps {
  open: boolean;
  models: DeviceModel[];
  onClose: () => void;
  createModel: (data: Partial<DeviceModel>) => Promise<void>;
  updateModel: (id: number, data: Partial<DeviceModel>) => Promise<void>;
  removeModel: (id: number) => Promise<void>;
}

/** 设备型号管理弹窗（独立组件，降低 DeviceList 复杂度） */
export default function ModelManageModal({ open, models, onClose, createModel, updateModel, removeModel }: ModelManageModalProps) {
  const [editingModel, setEditingModel] = useState<DeviceModel | null>(null);
  const [searchText, setSearchText] = useState('');
  const [form] = Form.useForm();

  useEffect(() => {
    if (open) {
      setEditingModel(null);
      setSearchText('');
      form.resetFields();
    }
  }, [open, form]);

  const filteredModels = useMemo(() =>
    models.filter(m => m.name.toLowerCase().includes(searchText.toLowerCase())),
    [models, searchText],
  );

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

  const showModelModal = (model?: DeviceModel) => {
    if (model) {
      setEditingModel(model);
      form.setFieldsValue(model);
    } else {
      setEditingModel(null);
      form.resetFields();
    }
  };

  const handleSubmit = async (values: Partial<DeviceModel>) => {
    if (editingModel) {
      await updateModel(editingModel.id, values);
      setEditingModel(null);
      form.resetFields();
    } else {
      await createModel(values);
      form.resetFields();
    }
  };

  return (
    <Modal
      title="型号管理"
      open={open}
      onCancel={onClose}
      width={700}
      footer={null}
    >
      <div className="model-search-bar">
        <Input
          className="model-search-input"
          placeholder="搜索型号..."
          value={searchText}
          onChange={e => setSearchText(e.target.value)}
        />
        <Button type="primary" onClick={() => showModelModal()}>
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
        <Form form={form} layout="inline" onFinish={handleSubmit} style={{ gap: 8, flexWrap: 'wrap' }}>
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '必填' }]} style={{ marginBottom: 8 }}>
            <Input style={{ width: 140 }} />
          </Form.Item>
          <Form.Item name="manufacturer" label="厂家" style={{ marginBottom: 8 }}>
            <Input style={{ width: 120 }} />
          </Form.Item>
          <Form.Item name="type" label="类型" rules={[{ required: true, message: '必填' }]} style={{ marginBottom: 8 }}>
            <Select options={DEVICE_TYPE_OPTIONS} style={{ width: 110 }} />
          </Form.Item>
          <Form.Item name="height_u" label="高度U" rules={[{ required: true, message: '必填' }]} style={{ marginBottom: 8 }}>
            <InputNumber min={1} max={50} style={{ width: 80 }} />
          </Form.Item>
          <Form.Item style={{ marginBottom: 8 }}>
            <Space>
              <Button type="primary" htmlType="submit">{editingModel ? '保存' : '创建'}</Button>
              {editingModel && (
                <Button onClick={() => showModelModal()}>取消编辑</Button>
              )}
            </Space>
          </Form.Item>
        </Form>
      </div>
    </Modal>
  );
}
