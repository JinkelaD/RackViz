import { useCallback, useRef, useState } from 'react';

/** 撤销栈容量上限（设计 §8-5：上限 20 条、FIFO 淘汰） */
export const UNDO_STACK_LIMIT = 20;

/** 一个可撤销操作：展示用 label + 逆操作 */
export interface UndoAction {
  /** 展示文案，如 `删除设备「交换机A」` */
  label: string;
  /** 逆操作（异步） */
  undo: () => Promise<void> | void;
}

/** 栈内条目：附带单调递增序号，作为稳定标识（React key / 精确移除） */
export interface UndoEntry extends UndoAction {
  seq: number;
}

export interface UseUndoStackReturn {
  /** 推入一个可撤销操作（超出上限自动 FIFO 淘汰最旧） */
  push: (action: UndoAction) => UndoEntry;
  /** 撤销最近一次操作；无可撤销项时返回 null（无副作用） */
  undo: () => Promise<UndoEntry | null>;
  /** 清空整个栈 */
  clear: () => void;
  canUndo: boolean;
  size: number;
}

/**
 * 内存撤销栈（N-11，设计 §8-5）。
 * - **仅前端内存、会话级**：刷新/重启即清空，不做任何持久化。
 * - **上限 20 条、FIFO 淘汰**：超出容量丢弃最旧条目。
 * - **撤销 = 栈顶（最近一次）操作的逆操作**；无可撤销项时静默返回 null。
 * - 逆操作失败时**不弹出该条目**（保留在栈顶，用户可重试），由调用方负责错误提示。
 */
export function useUndoStack(): UseUndoStackReturn {
  const [entries, setEntries] = useState<UndoEntry[]>([]);
  // 镜像最新栈，供 undo 读取（避免闭包捕获过期值）
  const entriesRef = useRef<UndoEntry[]>(entries);
  entriesRef.current = entries;
  const seqRef = useRef(0);

  const push = useCallback((action: UndoAction): UndoEntry => {
    const entry: UndoEntry = { ...action, seq: ++seqRef.current };
    setEntries(prev => {
      const next = [...prev, entry];
      // FIFO 淘汰：容量上限，丢弃最旧
      return next.length > UNDO_STACK_LIMIT ? next.slice(next.length - UNDO_STACK_LIMIT) : next;
    });
    return entry;
  }, []);

  const undo = useCallback(async (): Promise<UndoEntry | null> => {
    const list = entriesRef.current;
    if (list.length === 0) return null; // 无可撤销操作 → 无副作用
    const entry = list[list.length - 1];
    // 先执行逆操作；成功后再移除（失败保留栈顶，便于重试）
    await entry.undo();
    setEntries(prev => prev.filter(e => e.seq !== entry.seq));
    return entry;
  }, []);

  const clear = useCallback(() => setEntries([]), []);

  return { push, undo, clear, canUndo: entries.length > 0, size: entries.length };
}
