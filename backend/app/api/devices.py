from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy.orm import Session, joinedload
from typing import List, Optional
from ..core.database import get_db
from ..models.device import Device
from ..schemas.device import DeviceCreate, DeviceUpdate, DeviceResponse
from ..services.icmp import ping as icmp_ping

router = APIRouter(prefix="/devices", tags=["devices"])

@router.get("/", response_model=List[DeviceResponse])
def list_devices(
    rack_id: Optional[int] = Query(None),
    search: Optional[str] = Query(None),
    db: Session = Depends(get_db),
):
    q = db.query(Device).options(joinedload(Device.rack), joinedload(Device.device_model))
    if rack_id is not None:
        q = q.filter(Device.rack_id == rack_id)
    if search:
        q = q.filter(Device.name.ilike(f"%{search}%"))
    return q.all()

@router.post("/", response_model=DeviceResponse, status_code=201)
def create_device(data: DeviceCreate, db: Session = Depends(get_db)):
    device = Device(**data.model_dump())
    db.add(device)
    db.commit()
    db.refresh(device)
    return device

@router.get("/{device_id}", response_model=DeviceResponse)
def get_device(device_id: int, db: Session = Depends(get_db)):
    device = db.query(Device).options(
        joinedload(Device.rack), joinedload(Device.device_model)
    ).filter(Device.id == device_id).first()
    if not device:
        raise HTTPException(status_code=404, detail="Device not found")
    return device

@router.put("/{device_id}", response_model=DeviceResponse)
def update_device(device_id: int, data: DeviceUpdate, db: Session = Depends(get_db)):
    device = db.query(Device).filter(Device.id == device_id).first()
    if not device:
        raise HTTPException(status_code=404, detail="Device not found")
    for key, val in data.model_dump(exclude_unset=True).items():
        setattr(device, key, val)
    db.commit()
    db.refresh(device)
    return device

@router.delete("/{device_id}", status_code=204)
def delete_device(device_id: int, db: Session = Depends(get_db)):
    device = db.query(Device).filter(Device.id == device_id).first()
    if not device:
        raise HTTPException(status_code=404, detail="Device not found")
    db.delete(device)
    db.commit()

@router.post("/refresh-status")
def refresh_device_status(db: Session = Depends(get_db)):
    devices = db.query(Device).all()
    results = []
    for device in devices:
        if device.ip_addresses:
            first_ip = device.ip_addresses.split(";")[0].strip()
            latency = icmp_ping(first_ip)
            new_status = "online" if latency is not None else "offline"
        else:
            new_status = "unconfigured"
        if device.status != new_status:
            device.status = new_status
        results.append({"id": device.id, "status": new_status})
    db.commit()
    return results