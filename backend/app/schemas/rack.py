from pydantic import BaseModel, Field
from typing import Optional

class RackBase(BaseModel):
    name: str = Field(..., min_length=1, max_length=100)
    height_u: int = Field(default=42, ge=6, le=42)
    row: int = Field(default=0, ge=0, le=23)
    col: int = Field(default=0, ge=0, le=23)
    view: str = "front"

class RackCreate(RackBase):
    pass

class RackUpdate(BaseModel):
    name: Optional[str] = None
    height_u: Optional[int] = None
    row: Optional[int] = None
    col: Optional[int] = None
    view: Optional[str] = None

class RackResponse(RackBase):
    id: int
    model_config = {"from_attributes": True}