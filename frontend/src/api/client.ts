const BASE = '/api';

async function request<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${url}`, {
    headers: { 'Content-Type': 'application/json', ...options?.headers },
    ...options,
  });
  if (!res.ok) {
    const err = await res.text();
    throw new Error(err || `HTTP ${res.status}`);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

export const deviceModelApi = {
  list: () => request<import('../types').DeviceModel[]>('/device-models/'),
  create: (data: Partial<import('../types').DeviceModel>) =>
    request<import('../types').DeviceModel>('/device-models/', { method: 'POST', body: JSON.stringify(data) }),
  update: (id: number, data: Partial<import('../types').DeviceModel>) =>
    request<import('../types').DeviceModel>(`/device-models/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  delete: (id: number) =>
    request<void>(`/device-models/${id}`, { method: 'DELETE' }),
};

export const roomApi = {
  list: () => request<import('../types').Room[]>('/rooms/'),
  create: (data: Partial<import('../types').Room>) =>
    request<import('../types').Room>('/rooms/', { method: 'POST', body: JSON.stringify(data) }),
  update: (id: number, data: Partial<import('../types').Room>) =>
    request<import('../types').Room>(`/rooms/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  delete: (id: number) =>
    request<void>(`/rooms/${id}`, { method: 'DELETE' }),
};

export const rackApi = {
  list: () => request<import('../types').Rack[]>('/racks/'),
  create: (data: Partial<import('../types').Rack>) =>
    request<import('../types').Rack>('/racks/', { method: 'POST', body: JSON.stringify(data) }),
  update: (id: number, data: Partial<import('../types').Rack>) =>
    request<import('../types').Rack>(`/racks/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  delete: (id: number) =>
    request<void>(`/racks/${id}`, { method: 'DELETE' }),
};

export const deviceApi = {
  list: (params?: { rack_id?: number; search?: string }) => {
    const qs = new URLSearchParams();
    if (params?.rack_id !== undefined) qs.set('rack_id', String(params.rack_id));
    if (params?.search) qs.set('search', params.search);
    return request<import('../types').Device[]>(`/devices/${qs.toString() ? '?' + qs : ''}`);
  },
  create: (data: Partial<import('../types').Device>) =>
    request<import('../types').Device>('/devices/', { method: 'POST', body: JSON.stringify(data) }),
  update: (id: number, data: Partial<import('../types').Device>) =>
    request<import('../types').Device>(`/devices/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  delete: (id: number) =>
    request<void>(`/devices/${id}`, { method: 'DELETE' }),
  refreshStatus: () =>
    request<Array<{ id: number; status: string }>>('/devices/refresh-status', { method: 'POST' }),
};

export const exportApi = {
  downloadRacksExcel: () => `${BASE}/export/racks.xlsx`,
  importExcel: (file: File) => {
    const form = new FormData();
    form.append('file', file);
    return fetch(`${BASE}/export/import`, { method: 'POST', body: form }).then(r => r.json());
  },
};