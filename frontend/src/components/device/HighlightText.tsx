import { Fragment } from 'react';
import { splitHighlight } from '../../utils/highlight';

interface HighlightTextProps {
  /** 单元格文本（为空时渲染占位 `-`，与既有列渲染保持一致） */
  text: string | null | undefined;
  /** 当前搜索关键词（不区分大小写；空则不包裹） */
  keyword?: string | null;
  className?: string;
}

/**
 * 关键词高亮渲染组件（N-03）。
 * 命中片段用 `<mark>` 包裹；仅作用于「当前页返回行」，与分页/排序天然一致不错位。
 */
export default function HighlightText({ text, keyword, className }: HighlightTextProps) {
  const value = text ?? '';

  // 空文本：沿用既有占位符
  if (!value) {
    return <span className={className}>-</span>;
  }

  const segments = splitHighlight(value, keyword);
  const hasMatch = segments.some(s => s.match);

  if (!hasMatch) {
    return <span className={className}>{value}</span>;
  }

  return (
    <span className={className}>
      {segments.map((seg, index) =>
        seg.match
          ? <mark key={index}>{seg.text}</mark>
          : <Fragment key={index}>{seg.text}</Fragment>,
      )}
    </span>
  );
}
