/**
 * 对象字段工具（C2a：替代撤销栈深拷贝中的裸 Record 收窄断言）。
 *
 * 用 `Object.entries` 运行时反射按白名单 key 提取字段——无断言、无类型逃逸；
 * 调用侧以 `Object.assign(inverse, pickFields(...))` 回填 `Partial<T>` 目标对象。
 */
export function pickFields(obj: object, keys: readonly string[]): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  const wanted = new Set(keys);
  for (const [k, v] of Object.entries(obj)) {
    if (wanted.has(k)) out[k] = v;
  }
  return out;
}
