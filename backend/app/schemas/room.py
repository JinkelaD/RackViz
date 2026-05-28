from pydantic import BaseModel, Field
from typing import Optional

class RoomBase(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    location: str = ""
    sort_order: int = 0

class RoomCreate(RoomBase):
    pass

class RoomUpdate(BaseModel):
    name: Optional[str] = None
    location: Optional[str] = None
    sort_order: Optional[int] = None

class RoomResponse(RoomBase):
    id: int
    model_config = {"from_attributes": True}
