from sqlalchemy import Column, Integer, String
from ..core.database import Base

class DeviceModel(Base):
    __tablename__ = "device_models"

    id = Column(Integer, primary_key=True, index=True)
    name = Column(String(100), nullable=False)
    manufacturer = Column(String(100), default="")
    type = Column(String(50), nullable=False, default="server")
    height_u = Column(Integer, nullable=False, default=1)
    power_watt = Column(Integer, default=0)