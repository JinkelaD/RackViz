import { useMemo } from 'react';
import { createPortal } from 'react-dom';
import { Button, Modal, App } from 'antd';
import { PrinterOutlined } from '@ant-design/icons';
import type { Device, DeviceModel, Rack, Room } from '../../types';
import PrintLabelCell from './PrintLabelCell';

/** 每页标签数：A4 纵向 3 列 × 8 行（@page margin 10mm，标签区 63×34mm） */
const COLS = 3;
const ROWS = 8;
const PER_PAGE = COLS * ROWS;

interface BatchPrintModalProps {
  open: boolean;
  /** 待打印设备（台账选区，单选/多选同一流程） */
  devices: Device[];
  models: DeviceModel[];
  racks: Rack[];
  rooms: Room[];
  onClose: () => void;
}

/**
 * 二维码标签批量打印（B4）：
 * - 唯一打印通道：单选 1 台时网格仅左上角 1 个标签，多选时按序填充、每 24 个一页；
 * - 排序：机房 sort_order → 机柜 row/col → U 位升序，未上架排最后，贴签无回找；
 * - 打印输出经 body 级 portal（.batch-print-root）：屏幕下隐藏，@media print 下
 *   由全局规则隐藏 #root 与 antd 浮层后仅保留本根，无需额外打印模式开关。
 */
export default function BatchPrintModal({
  open,
  devices,
  models,
  racks,
  rooms,
  onClose,
}: BatchPrintModalProps) {
  const { message } = App.useApp();

  const modelById = useMemo(() => new Map(models.map(m => [m.id, m])), [models]);
  const rackById = useMemo(() => new Map(racks.map(r => [r.id, r])), [racks]);
  const roomOrder = useMemo(() => new Map(rooms.map(r => [r.id, r.sort_order])), [rooms]);

  /** 贴签顺序：机房 → 机柜位置 → U 位；未上架排最后（组内按名称） */
  const sorted = useMemo(() => {
    return [...devices].sort((a, b) => {
      const ra = a.rack_id != null ? rackById.get(a.rack_id) : undefined;
      const rb = b.rack_id != null ? rackById.get(b.rack_id) : undefined;
      if (!ra && !rb) return a.name.localeCompare(b.name);
      if (!ra) return 1;
      if (!rb) return -1;
      const ro = (roomOrder.get(ra.room_id ?? -1) ?? 0) - (roomOrder.get(rb.room_id ?? -1) ?? 0);
      if (ro !== 0) return ro;
      if (ra.row !== rb.row) return ra.row - rb.row;
      if (ra.col !== rb.col) return ra.col - rb.col;
      return (a.start_u ?? 0) - (b.start_u ?? 0);
    });
  }, [devices, rackById, roomOrder]);

  const pageCount = Math.max(1, Math.ceil(sorted.length / PER_PAGE));
  const unmountedCount = sorted.filter(d => d.rack_id == null).length;

  /** 渲染所有分页 sheet；末页不足补空白格保持网格对齐 */
  const renderSheets = () => {
    const sheets = [];
    for (let p = 0; p < pageCount; p++) {
      const chunk = sorted.slice(p * PER_PAGE, (p + 1) * PER_PAGE);
      const slots = [];
      for (let i = 0; i < PER_PAGE; i++) {
        const d = chunk[i];
        slots.push(
          <div className="print-label-slot" key={d ? d.id : `blank-${p}-${i}`}>
            {d && (
              <PrintLabelCell
                device={d}
                rackName={d.rack_id != null ? rackById.get(d.rack_id)?.name ?? null : null}
                modelName={d.device_model_id != null ? modelById.get(d.device_model_id)?.name : undefined}
              />
            )}
          </div>,
        );
      }
      sheets.push(
        <div className="batch-sheet" key={p}>
          <div className="batch-sheet-grid">{slots}</div>
        </div>,
      );
    }
    return sheets;
  };

  const handlePrint = () => {
    try {
      window.print();
    } catch (err) {
      message.error(`打印失败：${err instanceof Error ? err.message : String(err)}`);
    }
  };

  return (
    <>
      <Modal
        open={open}
        onCancel={onClose}
        width={880}
        title={`打印二维码标签（${sorted.length} 台 · ${pageCount} 页）`}
        footer={[
          <Button key="cancel" onClick={onClose}>取消</Button>,
          <Button key="print" type="primary" icon={<PrinterOutlined />} onClick={handlePrint}>
            打印（{pageCount} 页 A4）
          </Button>,
        ]}
      >
        {unmountedCount > 0 && (
          <p className="batch-print-warn">
            ⚠️ 其中 {unmountedCount} 台未上架，标签将显示「未上架」；建议先在机柜视图完成上架再打印。
          </p>
        )}
        <p className="batch-print-hint">
          标签按 机房 → 机柜 → U 位 排序，浅灰虚线为裁切线；A4 纵向每页 {PER_PAGE} 张，打印对话框中亦可「另存为 PDF」合并输出。
        </p>
        <div className="batch-preview">{renderSheets()}</div>
      </Modal>
      {open && createPortal(
        <div className="batch-print-root">{renderSheets()}</div>,
        document.body,
      )}
    </>
  );
}
