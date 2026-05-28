from fastapi import APIRouter
from .device_models import router as device_models_router
from .racks import router as racks_router
from .devices import router as devices_router
from .export import router as export_router
from .rooms import router as rooms_router

api_router = APIRouter()
api_router.include_router(device_models_router)
api_router.include_router(racks_router)
api_router.include_router(devices_router)
api_router.include_router(export_router)
api_router.include_router(rooms_router)