/**
 * 高亮分割纯函数（N-03）。
 *
 * 设计要点：
 * - **大小写不敏感**：用 `gi` 正则整体匹配，命中判定用 `toLowerCase` 比较，保留原文大小写。
 * - **正则元字符转义**：用户输入可能含 `.` `*` `(` `[` `\` 等，必须先 `escapeRegExp`，
 *   否则 `new RegExp(kw)` 会抛异常或产生误匹配（如 `a.b` 命中 `axb`）。
 * - **空关键词**：返回原文单段（不包裹 `<mark>`）。
 * - 返回分段数组，每段带 `match` 标记，供渲染层包 `<mark>`。
 *   （本文件为纯函数，无 React 依赖；扩展名沿用设计约定的 .tsx。）
 */

export interface HighlightSegment {
  /** 文本片段（原文大小写） */
  text: string;
  /** 是否为命中片段 */
  match: boolean;
}

/** 转义正则元字符，使字面量字符串可安全用于 `new RegExp` */
export function escapeRegExp(input: string): string {
  return input.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * 按关键词把一个字符串切成「命中 / 未命中」片段数组。
 * @param text 原始文本
 * @param keyword 关键词（可为空）
 */
export function splitHighlight(text: string, keyword?: string | null): HighlightSegment[] {
  const source = text ?? '';
  const kw = (keyword ?? '').trim();

  // 空文本或空关键词 → 原文单段
  if (!source || !kw) {
    return [{ text: source, match: false }];
  }

  // 捕获组使 split 结果保留分隔符（命中片段本身）
  const re = new RegExp(`(${escapeRegExp(kw)})`, 'gi');
  const parts = source.split(re);
  const needle = kw.toLowerCase();

  const segments: HighlightSegment[] = [];
  for (const part of parts) {
    if (part === '') continue;
    segments.push({ text: part, match: part.toLowerCase() === needle });
  }

  // 极端兜底：split 结果为空时返回原文
  return segments.length > 0 ? segments : [{ text: source, match: false }];
}
