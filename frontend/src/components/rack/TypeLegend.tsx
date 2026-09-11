import { DEVICE_TYPES, DEVICE_TYPE_LABELS } from '../../constants/labels';

/**
 * 机柜类型颜色图例：色块 + 中文类型名，色值引用 CSS 变量（--type-<key>），
 * 与设备块/图标/徽章配色单一来源（styles/global.css 的 type-color-map）。
 */
export default function TypeLegend() {
  return (
    <div className="type-legend" aria-label="设备类型图例">
      <span className="type-legend-title">设备类型</span>
      <div className="type-legend-items">
        {DEVICE_TYPES.map(t => (
          <span key={t} className="type-legend-item" title={DEVICE_TYPE_LABELS[t]}>
            <span
              className="type-legend-swatch"
              style={{ background: `var(--type-${t})` }}
            />
            {DEVICE_TYPE_LABELS[t]}
          </span>
        ))}
      </div>
    </div>
  );
}
