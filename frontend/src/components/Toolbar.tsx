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
}

export default function Toolbar({
  mode, onModeChange, view, onViewChange,
  zoom, onZoomIn, onZoomOut, onZoomReset,
  searchQuery, onSearchChange,
}: Props) {
  return (
    <div className="toolbar">
      <div className="toolbar-group">
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
      <div className="toolbar-divider" />
      <div className="toolbar-group">
        <button className="toolbar-btn" onClick={onZoomOut}>−</button>
        <button className="toolbar-btn" onClick={onZoomReset}>{zoom}%</button>
        <button className="toolbar-btn" onClick={onZoomIn}>+</button>
      </div>
      <div className="toolbar-divider" />
      <div className="toolbar-group">
        {(['front', 'rear'] as ViewMode[]).map(v => (
          <button
            key={v}
            className={`toolbar-btn ${view === v ? 'active' : ''}`}
            onClick={() => onViewChange(v)}
          >
            {v === 'front' ? '正面' : '背面'}
          </button>
        ))}
      </div>
      <div className="toolbar-search">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
        <input
          type="text"
          placeholder="搜索设备..."
          value={searchQuery}
          onChange={e => onSearchChange(e.target.value)}
        />
        <kbd>Ctrl+F</kbd>
      </div>
    </div>
  );
}