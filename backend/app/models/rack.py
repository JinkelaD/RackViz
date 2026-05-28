from sqlalchemy import Column, Integer, String, ForeignKey
from ..core.database import Base

class Rack(Base):
    __tablename__ = "racks"

    id = Column(Integer, primary_key=True, index=True)
    name = Column(String(100), nullable=False)
    height_u = Column(Integer, nullable=False, default=42)
    row = Column(Integer, default=0)
    col = Column(Integer, default=0)
    view = Column(String(10), default="front")
    sort_order = Column(Integer, default=0)
    room_id = Column(Integer, ForeignKey("rooms.id"), nullable=True, default=None)
