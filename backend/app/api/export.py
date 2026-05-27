from fastapi import APIRouter, Depends
from fastapi.responses import StreamingResponse
from sqlalchemy.orm import Session
from ..core.database import get_db
from ..services.excel_export import export_racks_excel

router = APIRouter(prefix="/export", tags=["export"])

@router.get("/racks.xlsx")
def download_racks_excel(db: Session = Depends(get_db)):
    output = export_racks_excel(db)
    return StreamingResponse(
        output,
        media_type="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        headers={"Content-Disposition": "attachment; filename=racks.xlsx"},
    )