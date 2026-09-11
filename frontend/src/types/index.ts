export interface DeviceModel {
  id: number;
  name: string;
  manufacturer: string;
  type: DeviceType;
  height_u: number;
  power_watt: number;
}

export type DeviceType = 'server' | 'switch' | 'router' | 'storage' | 'nas' | 'security' | 'loadbalancer' | 'other';

export interface Room {
  id: number;
  name: string;
  location: string;
  sort_order: number;
}

export interface Rack {
  id: number;
  name: string;
  height_u: number;
  row: number;
  col: number;
  view: 'front' | 'rear';
  sort_order: number;
  room_id: number | null;
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
  asset_no: string;
  department: string;
  owner: string;
  function: string;
  purchase_date: string | null;
  warranty_expire: string | null;
  status: 'online' | 'offline' | 'unconfigured';
  power_watt: number;
  /** 设备固有高度（U）；下架后保留，重上架不再退化为 1U */
  height_u: number;
}

export type EditMode = 'drag' | 'delete' | 'edit';
export type ViewMode = 'front' | 'rear';