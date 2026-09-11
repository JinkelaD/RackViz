interface Props {
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  onAddRack: () => void;
  /** 导出机柜部署图（全部机柜） */
  onExportRackPlan: () => void;
  /** 导出选中单机柜部署图；disabled 当无选中机柜 */
  onExportSingleRack: () => void;
  singleRackExportDisabled?: boolean;
  deviceCount: number;
  onlineCount: number;
  totalUUsed: number;
  totalU: number;
}

export default function StatusBar({
  zoom, onZoomIn, onZoomOut, onZoomReset,
  searchQuery, onSearchChange,
  onAddRack, onExportRackPlan, onExportSingleRack, singleRackExportDisabled,
  deviceCount, onlineCount, totalUUsed, totalU,
}: Props) {
  const uPct = totalU > 0 ? Math.round((totalUUsed / totalU) * 100) : 0;

  return (
    <div className="statusbar">
      <div className="statusbar-left">
        <button className="statusbar-btn accent" onClick={onAddRack} title="添加机柜">
          + 机柜
        </button>

        <div className="statusbar-segment">
          <button className="statusbar-btn" onClick={onExportRackPlan} title="导出机柜部署图 Excel">
            部署图
          </button>
          <button
            className="statusbar-btn"
            onClick={onExportSingleRack}
            disabled={singleRackExportDisabled}
            title={singleRackExportDisabled ? '请先点击选中一个机柜' : '导出选中机柜 Excel'}
          >
            单机柜
          </button>
        </div>

        <div className="statusbar-segment">
          <button className="statusbar-btn" onClick={onZoomOut} title="缩小">−</button>
          <button className="statusbar-btn zoom" onClick={onZoomReset}>{zoom}%</button>
          <button className="statusbar-btn" onClick={onZoomIn} title="放大">+</button>
        </div>

        <div className="statusbar-segment statusbar-search">
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
            <button className="statusbar-search-clear" onClick={() => onSearchChange('')}>
              ×
            </button>
          )}
        </div>
      </div>

      <div className="statusbar-right">
        <span className="statusbar-item">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="6" x2="22" y2="6"/><line x1="2" y1="10" x2="22" y2="10"/><line x1="2" y1="14" x2="22" y2="14"/></svg>
          {deviceCount} 设备
          <span className="statusbar-highlight">· {onlineCount} 开机</span>
        </span>
        <span className="statusbar-sep" />
        <span className="statusbar-item">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="8" x2="22" y2="8"/></svg>
          {totalUUsed}/{totalU}U
          <span className={`statusbar-pct ${uPct > 85 ? 'warn' : ''}`}>{uPct}%</span>
        </span>
      </div>
    </div>
  );
}
