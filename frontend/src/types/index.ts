export interface DeviceModel {
  id: number;
  name: string;
  manufacturer: string;
  type: DeviceType;
  height_u: number;
  power_watt: number;
}

export type DeviceType = 'server' | 'switch' | 'router' | 'storage' | 'pdu' | 'patch';

export interface Rack {
  id: number;
  name: string;
  height_u: number;
  row: number;
  col: number;
  view: 'front' | 'rear';
}

export interface Device {
  id: number;
  name: string;
  device_model_id: number | null;
  rack_id: number | null;
  start_u: number | null;
  end_u: number | null;
  ip_addresses: string;
  serial_no: string;
  function: string;
  purchase_date: string | null;
  warranty_expire: string | null;
  status: 'online' | 'offline' | 'unconfigured';
  power_watt: number;
}

export type EditMode = 'drag' | 'delete' | 'edit';
export type ViewMode = 'front' | 'rear';