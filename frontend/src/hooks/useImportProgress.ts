import { useCallback, useEffect, useRef, useState } from 'react';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { ImportProgress } from '../types';
import { onImportProgress } from '../tauri-api';

export interface UseImportProgressReturn {
  /** 最近一次进度事件（null 表示尚未开始） */
  progress: ImportProgress | null;
  /** 开始监听（导入前调用）：重置进度并挂载 `import://progress` 监听 */
  begin: () => Promise<void>;
  /** 结束监听（导入后务必在 finally 中调用，避免监听泄漏，§8-8） */
  end: () => void;
}

/**
 * 导入进度 Hook（N-06）。
 * 封装 `listen('import://progress')`，确保「同一时刻最多一个监听」，
 * 并在 begin 重置进度、end/卸载时 unlisten，避免事件回调累积。
 */
export function useImportProgress(): UseImportProgressReturn {
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const end = useCallback(() => {
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
  }, []);

  const begin = useCallback(async () => {
    // 防御：重复 begin 时先释放旧监听
    end();
    setProgress({ processed: 0, total: 0, phase: 'parsing' });
    unlistenRef.current = await onImportProgress((p) => setProgress(p));
  }, [end]);

  // 组件卸载时兜底 unlisten
  useEffect(() => () => end(), [end]);

  return { progress, begin, end };
}
