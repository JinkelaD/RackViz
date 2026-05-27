import { useState } from 'react';
import { Table, Button, Input, Tag, Space, Modal, Form, InputNumber, Select } from 'antd';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { DeviceModel } from '../types';

export default function DeviceModelList() {
  const { models, loading, create, update, remove } = useDeviceModels();
  const [searchText, setSearchText] = useState('');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<DeviceModel | null>(null);
  const [form] = Form.useForm();

  const deviceTypes = [
    { value: 'server', label: '服务器' },
    { value: 'switch', label: '交换机' },
    { value: 'router', label: '路由器' },
    { value: 'storage', label: '存储' },
    { value: 'pdu', label: 'PDU' },
    { value: 'patch', label: '配线架' },
  ];

  const getTypeTag = (type: string) => {
    const colors: Record<string, string> = {
      server: 'blue',
      switch: 'cyan',
      router: 'gold',
      storage: 'purple',
      pdu: 'red',
      patch: 'default',
    };
    const labels: Record<string, string> = {
      server: '服务器',
      switch: '交换机',
      router: '路由器',
      storage: '存储',
      pdu: 'PDU',
      patch: '配线架',
    };
    return <Tag color={colors[type]}>{labels[type]}</Tag>;
  };

  const filteredModels = models.filter(m => 
    m.name.toLowerCase().includes(searchText.toLowerCase())
  );

  const columns = [
    {
      title: '设备型号',
      dataIndex: 'name',
      key: 'name',
      width: 200,
    },
    {
      title: '厂家',
      dataIndex: 'manufacturer',
      key: 'manufacturer',
      width: 120,
      render: (val: string) => val || '-',
    },
    {
      title: '类型',
      dataIndex: 'type',
      key: 'type',
      width: 100,
      render: (type: string) => getTypeTag(type),
    },
    {
      title: '高度',
      dataIndex: 'height_u',
      key: 'height_u',
      width: 80,
      render: (u: number) => `${u}U`,
    },
    {
      title: '功率',
      dataIndex: 'power_watt',
      key: 'power',
      width: 80,
      render: (watt: number) => `${watt}W`,
    },
    {
      title: '操作',
      key: 'action',
      width: 150,
      render: (_, record: DeviceModel) => (
        <Space>
          <Button size="small" onClick={() => {
            setEditingModel(record);
            form.setFieldsValue(record);
            setIsModalOpen(true);
          }}>编辑</Button>
          <Button size="small" danger onClick={() => remove(record.id)}>删除</Button>
        </Space>
      ),
    },
  ];

  const handleSubmit = () => {
    form.validateFields().then(values => {
      if (editingModel) {
        update(editingModel.id, values);
      } else {
        create(values);
      }
      setIsModalOpen(false);
      setEditingModel(null);
      form.resetFields();
    });
  };

  const handleAdd = () => {
    setEditingModel(null);
    form.resetFields();
    setIsModalOpen(true);
  };

  return (
    <div style={{ padding: '16px', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
        <h2 style={{ margin: 0, fontSize: '16px', fontWeight: 600 }}>设备库</h2>
        <Space>
          <Input
            placeholder="搜索型号..."
            style={{ width: 200 }}
            value={searchText}
            onChange={e => setSearchText(e.target.value)}
          />
          <Button type="primary" onClick={handleAdd}>添加设备型号</Button>
        </Space>
      </div>
      <Table
        dataSource={filteredModels}
        columns={columns}
        loading={loading}
        rowKey="id"
        pagination={{ pageSize: 20 }}
        bordered={false}
      />

      <Modal
        title={editingModel ? '编辑设备型号' : '添加设备型号'}
        open={isModalOpen}
        onCancel={() => {
          setIsModalOpen(false);
          setEditingModel(null);
          form.resetFields();
        }}
        footer={null}
      >
        <Form form={form} layout="vertical" onFinish={handleSubmit}>
          <Form.Item
            name="name"
            label="型号名称"
            rules={[{ required: true, message: '请输入型号名称' }]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="manufacturer"
            label="厂家"
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="type"
            label="设备类型"
            rules={[{ required: true, message: '请选择设备类型' }]}
          >
            <Select options={deviceTypes} />
          </Form.Item>
          <Form.Item
            name="height_u"
            label="高度 (U)"
            rules={[{ required: true, message: '请输入高度' }]}
          >
            <InputNumber min={1} max={4} />
          </Form.Item>
          <Form.Item
            name="power_watt"
            label="功率 (W)"
          >
            <InputNumber min={0} />
          </Form.Item>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Button onClick={() => {
                setIsModalOpen(false);
                setEditingModel(null);
                form.resetFields();
              }}>取消</Button>
              <Button type="primary" htmlType="submit">确定</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}