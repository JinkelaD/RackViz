from pydantic import BaseModel, Field
from typing import Optional
from datetime import date

class DeviceModelBase(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    manufacturer: str = ""
    type: str = "server"
    height_u: int = Field(default=1, ge=1, le=4)
    power_watt: int = 0

class DeviceModelCreate(DeviceModelBase):
    pass

class DeviceModelUpdate(BaseModel):
    name: Optional[str] = None
    manufacturer: Optional[str] = None
    type: Optional[str] = None
    height_u: Optional[int] = None
    power_watt: Optional[int] = None

class DeviceModelResponse(DeviceModelBase):
    id: int
    model_config = {"from_attributes": True}