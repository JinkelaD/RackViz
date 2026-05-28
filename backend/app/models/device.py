from sqlalchemy import Column, Integer, String, Date, ForeignKey
from sqlalchemy.orm import relationship
from ..core.database import Base

class Device(Base):
    __tablename__ = "devices"

    id = Column(Integer, primary_key=True, index=True)
    name = Column(String(200), nullable=False)
    device_model_id = Column(Integer, ForeignKey("device_models.id"), nullable=True)
    rack_id = Column(Integer, ForeignKey("racks.id"), nullable=True)
    start_u = Column(Integer, nullable=True)
    end_u = Column(Integer, nullable=True)
    ip_addresses = Column(String(500), default="")
    serial_no = Column(String(200), default="")
    asset_no = Column(String(200), default="")
    department = Column(String(100), default="")
    owner = Column(String(100), default="")
    function = Column(String(500), default="")
    purchase_date = Column(Date, nullable=True)
    warranty_expire = Column(Date, nullable=True)
    status = Column(String(20), default="unconfigured")
    power_watt = Column(Integer, default=0)

    rack = relationship("Rack", backref="devices")
    device_model = relationship("DeviceModel", backref="devices")