from fastapi import APIRouter, Depends, HTTPException
from sqlalchemy.orm import Session
from typing import List
from ..core.database import get_db
from ..models.device_model import DeviceModel
from ..schemas.device_model import (
    DeviceModelCreate, DeviceModelUpdate, DeviceModelResponse
)

router = APIRouter(prefix="/device-models", tags=["device-models"])

@router.get("/", response_model=List[DeviceModelResponse])
def list_models(db: Session = Depends(get_db)):
    return db.query(DeviceModel).all()

@router.post("/", response_model=DeviceModelResponse, status_code=201)
def create_model(data: DeviceModelCreate, db: Session = Depends(get_db)):
    model = DeviceModel(**data.model_dump())
    db.add(model)
    db.commit()
    db.refresh(model)
    return model

@router.get("/{model_id}", response_model=DeviceModelResponse)
def get_model(model_id: int, db: Session = Depends(get_db)):
    model = db.query(DeviceModel).filter(DeviceModel.id == model_id).first()
    if not model:
        raise HTTPException(status_code=404, detail="Device model not found")
    return model

@router.put("/{model_id}", response_model=DeviceModelResponse)
def update_model(model_id: int, data: DeviceModelUpdate, db: Session = Depends(get_db)):
    model = db.query(DeviceModel).filter(DeviceModel.id == model_id).first()
    if not model:
        raise HTTPException(status_code=404, detail="Device model not found")
    for key, val in data.model_dump(exclude_unset=True).items():
        setattr(model, key, val)
    db.commit()
    db.refresh(model)
    return model

@router.delete("/{model_id}", status_code=204)
def delete_model(model_id: int, db: Session = Depends(get_db)):
    model = db.query(DeviceModel).filter(DeviceModel.id == model_id).first()
    if not model:
        raise HTTPException(status_code=404, detail="Device model not found")
    db.delete(model)
    db.commit()