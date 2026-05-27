from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from typing import List
from ..core.database import get_db
from ..models.rack import Rack
from ..schemas.rack import RackCreate, RackUpdate, RackResponse

router = APIRouter(prefix="/racks", tags=["racks"])

@router.get("/", response_model=List[RackResponse])
def list_racks(db: Session = Depends(get_db)):
    return db.query(Rack).order_by(Rack.row, Rack.col).all()

@router.post("/", response_model=RackResponse, status_code=201)
def create_rack(data: RackCreate, db: Session = Depends(get_db)):
    rack = Rack(**data.model_dump())
    db.add(rack)
    db.commit()
    db.refresh(rack)
    return rack

@router.get("/{rack_id}", response_model=RackResponse)
def get_rack(rack_id: int, db: Session = Depends(get_db)):
    rack = db.query(Rack).filter(Rack.id == rack_id).first()
    if not rack:
        raise HTTPException(status_code=404, detail="Rack not found")
    return rack

@router.put("/{rack_id}", response_model=RackResponse)
def update_rack(rack_id: int, data: RackUpdate, db: Session = Depends(get_db)):
    rack = db.query(Rack).filter(Rack.id == rack_id).first()
    if not rack:
        raise HTTPException(status_code=404, detail="Rack not found")
    for key, val in data.model_dump(exclude_unset=True).items():
        setattr(rack, key, val)
    db.commit()
    db.refresh(rack)
    return rack

@router.delete("/{rack_id}", status_code=204)
def delete_rack(rack_id: int, db: Session = Depends(get_db)):
    rack = db.query(Rack).filter(Rack.id == rack_id).first()
    if not rack:
        raise HTTPException(status_code=404, detail="Rack not found")
    db.delete(rack)
    db.commit()