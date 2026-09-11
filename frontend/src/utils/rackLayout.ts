/**
 * 机柜布局算法（纯函数）。
 * 坐标系：u=1 在底部，u=rackHeight 在顶部。
 */

/**
 * 在机柜中寻找可放下 deviceHeight 连续 U 位的最优空位。
 * - 先向下扫（targetU 之下）再向上扫（targetU 之上）
 * - 按离 targetU 的距离升序，距离相同取更靠上者
 * - 返回 { startU, endU }（闭区间）；无解返回 null
 */
export function findAvailableSlot(
  targetU: number,
  deviceHeight: number,
  rackHeight: number,
  existingDevices: { start_u: number | null; end_u: number | null; id: number }[],
  excludeDeviceId?: number,
): { startU: number; endU: number } | null {
  const filtered = excludeDeviceId != null
    ? existingDevices.filter(d => d.id !== excludeDeviceId)
    : existingDevices;

  const occupied = new Set<number>();
  filtered.forEach(d => {
    if (d.start_u != null && d.end_u != null) {
      for (let i = d.start_u; i <= d.end_u; i++) occupied.add(i);
    }
  });

  const clamp = (s: number) => Math.max(1, Math.min(s, rackHeight));

  const candidates: number[] = [];

  let s = clamp(targetU);
  while (s >= 1) {
    const e = s + deviceHeight - 1;
    if (e > rackHeight) { s--; continue; }
    let blocked = false;
    for (let i = s; i <= e; i++) {
      if (occupied.has(i)) { blocked = true; break; }
    }
    if (!blocked) candidates.push(s);
    s--;
  }

  s = clamp(targetU) + 1;
  while (s + deviceHeight - 1 <= rackHeight) {
    const e = s + deviceHeight - 1;
    let blocked = false;
    for (let i = s; i <= e; i++) {
      if (occupied.has(i)) { blocked = true; break; }
    }
    if (!blocked) candidates.push(s);
    s++;
  }

  if (candidates.length === 0) return null;

  candidates.sort((a, b) => {
    const da = Math.abs(a - targetU);
    const db = Math.abs(b - targetU);
    if (da !== db) return da - db;
    return b - a;
  });

  const best = candidates[0];
  return { startU: best, endU: best + deviceHeight - 1 };
}
