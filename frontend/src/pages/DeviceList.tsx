import { useState } from 'react';
import { Table, Button, Input, Tag, Space, Modal, Form, Select, InputNumber } from 'antd';
import { EditOutlined, PlusOutlined } from '@ant-design/icons';
import { useDevices } from '../hooks/useDevices';
import { useDeviceModels } from '../hooks/useDeviceModels';
import { useRacks } from '../hooks/useRacks';
import { Device, DeviceModel, Rack } from '../types';

export default function DeviceList() {
  const { devices, loading, remove, create, update } = useDevices();
  const { models } = useDeviceModels();
  const { racks } = useRacks();
  const [searchText, setSearchText] = useState('');
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [editingDevice, setEditingDevice] = useState<Device | null>(null);
  const [form] = Form.useForm();

  const showModal = (device?: Device) => {
    if (device) {
      setEditingDevice(device);
      form.setFieldsValue({
        ...device,
        device_model_id: device.device_model_id,
        rack_id: device.rack_id,
      });
    } else {
      setEditingDevice(null);
      form.resetFields();
    }
    setIsModalVisible(true);
  };

  const handleOk = async () => {
    try {
      const values = await form.validateFields();
      if (editingDevice) {
        await update(editingDevice.id, values);
      } else {
        await create(values);
      }
      setIsModalVisible(false);
      form.resetFields();
      setEditingDevice(null);
    } catch (error) {
      console.error('Form validation failed:', error);
    }
  };

  const handleCancel = () => {
    setIsModalVisible(false);
    form.resetFields();
    setEditingDevice(null);
  };

  const getSelectedModel = (modelId: number | undefined): DeviceModel | undefined => {
    return models.find(m => m.id === modelId);
  };

  const getSelectedRack = (rackId: number | undefined): Rack | undefined => {
    return racks.find(r => r.id === rackId);
  };

  const handleModelChange = (modelId: number | undefined) => {
    const model = getSelectedModel(modelId);
    if (model) {
      form.setFieldsValue({
        power_watt: model.power_watt,
      });
    }
  };

  const handleStartUChange = (startU: number | undefined) => {
    const modelId = form.getFieldValue('device_model_id');
    const model = getSelectedModel(modelId);
    if (model && startU) {
      form.setFieldsValue({
        end_u: startU + model.height_u - 1,
      });
    }
  };

  const getDeviceModelName = (modelId: number | null) => {
    if (modelId === null) return '-';
    return models.find(m => m.id === Number(modelId))?.name || '-';
  };

  const getRackName = (rackId: number | null) => {
    return racks.find(r => r.id === rackId)?.name || '-';
  };

  const getStatusTag = (status: string) => {
    const colors: Record<string, string> = {
      online: 'success',
      offline: 'error',
      unconfigured: 'default',
    };
    const labels: Record<string, string> = {
      online: '在线',
      offline: '离线',
      unconfigured: '未配置',
    };
    return <Tag color={colors[status]}>{labels[status]}</Tag>;
  };

  const filteredDevices = devices.filter(d => 
    d.name.toLowerCase().includes(searchText.toLowerCase())
  );

  const columns = [
    {
      title: '设备名称',
      dataIndex: 'name',
      key: 'name',
      width: 150,
    },
    {
      title: '型号',
      dataIndex: 'device_model_id',
      key: 'model',
      width: 180,
      render: (id: number | null) => getDeviceModelName(id),
    },
    {
      title: '机柜',
      dataIndex: 'rack_id',
      key: 'rack',
      width: 100,
      render: (id: number | null) => getRackName(id),
    },
    {
      title: '位置',
      key: 'position',
      width: 100,
      render: (_, record: Device) => 
        record.start_u && record.end_u ? `${record.start_u}-${record.end_u}U` : '-',
    },
    {
      title: 'IP',
      dataIndex: 'ip_addresses',
      key: 'ip',
      width: 120,
      render: (ips: string) => ips || '-',
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 80,
      render: (status: string) => getStatusTag(status),
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
      width: 120,
      render: (_, record: Device) => (
        <Space>
          <Button size="small" icon={<EditOutlined />} onClick={() => showModal(record)}>编辑</Button>
          <Button size="small" danger onClick={() => remove(record.id)}>删除</Button>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: '16px', height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
        <h2 style={{ margin: 0, fontSize: '16px', fontWeight: 600 }}>设备台账</h2>
        <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
          <Button type="primary" onClick={() => showModal()} icon={<PlusOutlined />}>
            添加设备
          </Button>
          <Input
            placeholder="搜索设备..."
            style={{ width: 200 }}
            value={searchText}
            onChange={e => setSearchText(e.target.value)}
          />
        </div>
      </div>
      <Table
        dataSource={filteredDevices}
        columns={columns}
        loading={loading}
        rowKey="id"
        pagination={{ pageSize: 20 }}
        bordered={false}
      />

      <Modal
        title={editingDevice ? '编辑设备' : '添加设备'}
        open={isModalVisible}
        onOk={handleOk}
        onCancel={handleCancel}
        width={600}
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="name"
            label="设备名称"
            rules={[{ required: true, message: '请输入设备名称' }]}
          >
            <Input placeholder="请输入设备名称" />
          </Form.Item>

          <Form.Item
            name="device_model_id"
            label="设备型号"
            rules={[{ required: true, message: '请选择设备型号' }]}
          >
            <Select 
              placeholder="请选择设备型号" 
              onChange={handleModelChange}
            >
              {models.map(model => (
                <Select.Option key={model.id} value={model.id}>
                  {model.name} ({model.type} · {model.height_u}U)
                </Select.Option>
              ))}
            </Select>
          </Form.Item>

          <Form.Item
            name="rack_id"
            label="所属机柜"
          >
            <Select placeholder="请选择机柜（可选）">
              <Select.Option value={null}>不放入机柜</Select.Option>
              {racks.map(rack => (
                <Select.Option key={rack.id} value={rack.id}>
                  {rack.name} ({rack.height_u}U)
                </Select.Option>
              ))}
            </Select>
          </Form.Item>

          <Form.Item
            name="start_u"
            label="起始U位"
          >
            <InputNumber 
              min={1} 
              max={100} 
              placeholder="起始U位" 
              onChange={handleStartUChange}
            />
          </Form.Item>

          <Form.Item
            name="end_u"
            label="结束U位"
          >
            <InputNumber min={1} max={100} placeholder="结束U位" disabled />
          </Form.Item>

          <Form.Item
            name="power_watt"
            label="功率(W)"
          >
            <InputNumber min={0} disabled />
          </Form.Item>

          <Form.Item
            name="ip_addresses"
            label="IP地址"
          >
            <Input placeholder="多个IP用逗号分隔" />
          </Form.Item>

          <Form.Item
            name="serial_no"
            label="序列号"
          >
            <Input placeholder="设备序列号" />
          </Form.Item>

          <Form.Item
            name="function"
            label="功能描述"
          >
            <Input placeholder="设备功能描述" />
          </Form.Item>

          <Form.Item
            name="purchase_date"
            label="采购日期"
          >
            <Input type="date" />
          </Form.Item>

          <Form.Item
            name="warranty_expire"
            label="质保到期"
          >
            <Input type="date" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}