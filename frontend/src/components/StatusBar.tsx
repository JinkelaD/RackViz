interface Props {
  zoom: number;
  rackCount: number;
  deviceCount: number;
  onlineCount: number;
  totalPower: number;
  mode: string;
}

export default function StatusBar({ zoom, rackCount, deviceCount, onlineCount, totalPower, mode }: Props) {
  return (
    <div className="statusbar">
      <span>缩放: {zoom}%</span>
      <span>|</span>
      <span>机柜: {rackCount} 个</span>
      <span>|</span>
      <span>设备: {deviceCount} 台 · 在线: {onlineCount}</span>
      <span>|</span>
      <span>总功率: {totalPower.toLocaleString()}W</span>
      <span style={{ marginLeft: 'auto' }}>
        {mode === 'drag' ? '拖拽模式' : mode === 'delete' ? '删除模式' : '编辑模式'} · Ctrl+滚轮缩放
      </span>
    </div>
  );
}