import { createContext, useCallback, useContext, useMemo, type ReactNode } from 'react';
import { App, Button } from 'antd';
import { useUndoStack, type UndoAction } from '../hooks/useUndoStack';
import { errorMessage } from '../tauri-api';

export interface UndoContextValue {
  /** 登记一个可撤销操作，并弹出全局 Toast（含「撤销」入口）（PRD 第 303 行） */
  pushUndo: (action: UndoAction) => void;
  /** 撤销最近一次操作（无可撤销项时静默无副作用） */
  undo: () => Promise<void>;
  canUndo: boolean;
  size: number;
}

const UndoContext = createContext<UndoContextValue | null>(null);

/**
 * 撤销栈 Provider（N-11）。
 * - 内存态、会话级、刷新即清空（见 `useUndoStack`）。
 * - 全局 Toast：每次登记可撤销操作后提示，并提供「撤销」按钮直达撤销。
 * - 必须置于 antd `<App>` 之内（`main.tsx` 的 `AntdApp`）以取用 `message`。
 */
export function UndoProvider({ children }: { children: ReactNode }) {
  const { message } = App.useApp();
  const { push, undo: undoStack, canUndo, size } = useUndoStack();

  const undo = useCallback(async () => {
    try {
      const entry = await undoStack();
      if (entry) {
        message.success(`已撤销：${entry.label}`);
      }
      // entry 为 null 时不提示（无副作用）
    } catch (err) {
      message.error(`撤销失败：${errorMessage(err)}`);
    }
  }, [undoStack, message]);

  const pushUndo = useCallback((action: UndoAction) => {
    push(action);
    message.open({
      type: 'info',
      duration: 4,
      content: (
        <span>
          {action.label}
          <Button
            type="link"
            size="small"
            style={{ marginLeft: 8, padding: 0 }}
            onClick={() => { void undo(); }}
          >
            撤销
          </Button>
        </span>
      ),
    });
  }, [push, undo, message]);

  const value = useMemo<UndoContextValue>(
    () => ({ pushUndo, undo, canUndo, size }),
    [pushUndo, undo, canUndo, size],
  );

  return <UndoContext.Provider value={value}>{children}</UndoContext.Provider>;
}

export function useUndo(): UndoContextValue {
  const ctx = useContext(UndoContext);
  if (!ctx) throw new Error('useUndo 必须在 <UndoProvider> 内使用');
  return ctx;
}
