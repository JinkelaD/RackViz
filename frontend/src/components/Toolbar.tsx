import { EditMode, ViewMode } from '../types';

interface Props {
  mode: EditMode;
  onModeChange: (m: EditMode) => void;
  view: ViewMode;
  onViewChange: (v: ViewMode) => void;
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  onAddRack: () => void;
}

export default function Toolbar({
  mode, onModeChange,
  zoom, onZoomIn, onZoomOut, onZoomReset,
  searchQuery, onSearchChange,
  onAddRack,
}: Props) {
  return (
    <div className="toolbar">
      <div className="toolbar-left">
        <button className="toolbar-btn accent" onClick={onAddRack} title="添加机柜">
          + 机柜
        </button>
        <div className="toolbar-segment">
          {(['drag', 'edit', 'delete'] as EditMode[]).map(m => (
            <button
              key={m}
              className={`toolbar-btn ${mode === m ? 'active' : ''} ${m === 'delete' ? 'danger' : ''}`}
              onClick={() => onModeChange(m)}
            >
              {m === 'drag' ? '↕ 拖拽' : m === 'edit' ? '✎ 编辑' : '✕ 删除'}
            </button>
          ))}
        </div>
      </div>

      <div className="toolbar-center">
        <div className="toolbar-segment">
          <button className="toolbar-btn" onClick={onZoomOut} title="缩小">−</button>
          <button className="toolbar-btn zoom" onClick={onZoomReset}>{zoom}%</button>
          <button className="toolbar-btn" onClick={onZoomIn} title="放大">+</button>
        </div>
      </div>

      <div className="toolbar-right">
        <div className="toolbar-segment toolbar-search-wrap">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
          </svg>
          <input
            type="text"
            placeholder="搜索..."
            value={searchQuery}
            onChange={e => onSearchChange(e.target.value)}
          />
          {searchQuery && (
            <button className="search-clear-inline" onClick={() => onSearchChange('')}>
              ×
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
