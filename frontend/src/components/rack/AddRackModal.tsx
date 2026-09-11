import { useState, useEffect } from 'react';
import type { Room } from '../../types';

interface AddRackModalProps {
  open: boolean;
  rooms: Room[];
  defaultRoomId: number | null;
  onClose: () => void;
  /** 确认创建：名称 / 高度(U) / 所属机房 */
  onCreate: (name: string, height: number, roomId: number | null) => void;
}

/** 新建机柜弹窗（纯手写模态，与页面内其它弹窗样式一致） */
export default function AddRackModal({ open, rooms, defaultRoomId, onClose, onCreate }: AddRackModalProps) {
  const [name, setName] = useState('');
  const [height, setHeight] = useState(42);
  const [roomId, setRoomId] = useState<number | null>(null);

  // 每次打开时把默认机房同步为当前选中机房
  useEffect(() => {
    if (open) {
      setRoomId(defaultRoomId);
    }
  }, [open, defaultRoomId]);

  if (!open) return null;

  const submit = () => {
    if (!name.trim()) return;
    onCreate(name.trim(), height, roomId);
    setName('');
    setHeight(42);
    setRoomId(null);
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          添加机柜
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        <div className="modal-body">
          <div className="form-group">
            <label className="form-label">机柜名称</label>
            <input
              className="form-input"
              type="text"
              value={name}
              onChange={e => setName(e.target.value)}
              placeholder="输入机柜名称"
              autoFocus
            />
          </div>
          <div className="form-group">
            <label className="form-label">机柜高度（U）</label>
            <input
              className="form-input"
              type="number"
              min={4}
              max={48}
              value={height}
              onChange={e => setHeight(Number(e.target.value))}
            />
          </div>
          <div className="form-group">
            <label className="form-label">所属机房</label>
            <select
              className="form-input"
              value={roomId || ''}
              onChange={e => setRoomId(e.target.value ? Number(e.target.value) : null)}
            >
              <option value="">选择机房</option>
              {rooms.map(room => (
                <option key={room.id} value={room.id}>{room.name}</option>
              ))}
            </select>
          </div>
        </div>
        <div className="modal-actions">
          <button className="btn" onClick={onClose}>取消</button>
          <button className="btn primary" onClick={submit}>确定</button>
        </div>
      </div>
    </div>
  );
}
