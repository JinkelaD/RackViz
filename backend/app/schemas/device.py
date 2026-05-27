from pydantic import BaseModel, Field
from typing import Optional
from datetime import date

class DeviceBase(BaseModel):
    name: str = Field(..., min_length=1, max_length=200)
    device_model_id: Optional[int] = None
    rack_id: Optional[int] = None
    start_u: Optional[int] = None
    end_u: Optional[int] = None
    ip_addresses: str = ""
    serial_no: str = ""
    function: str = ""
    purchase_date: Optional[date] = None
    warranty_expire: Optional[date] = None
    status: str = "unconfigured"
    power_watt: int = 0

class DeviceCreate(DeviceBase):
    pass

class DeviceUpdate(BaseModel):
    name: Optional[str] = None
    device_model_id: Optional[int] = None
    rack_id: Optional[int] = None
    start_u: Optional[int] = None
    end_u: Optional[int] = None
    ip_addresses: Optional[str] = None
    serial_no: Optional[str] = None
    function: Optional[str] = None
    purchase_date: Optional[date] = None
    warranty_expire: Optional[date] = None
    status: Optional[str] = None
    power_watt: Optional[int] = None

class DeviceResponse(DeviceBase):
    id: int
    model_config = {"from_attributes": True}