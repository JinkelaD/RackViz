from sqlalchemy import Column, Integer, String
from ..core.database import Base

class Room(Base):
    __tablename__ = "rooms"

    id = Column(Integer, primary_key=True, index=True)
    name = Column(String(100), nullable=False)
    location = Column(String(200), default="")
    sort_order = Column(Integer, default=0)
