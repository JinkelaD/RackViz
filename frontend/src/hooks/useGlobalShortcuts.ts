import {
  createContext,
  createElement,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  type ReactNode,
} from 'react';

/** 页面级快捷键处理器（按上下文按需提供，未定义的键不拦截） */
export interface ShortcutHandlers {
  /** Ctrl/Cmd + N —— 新建（台账：打开「添加设备」弹窗） */
  onNew?: () => void;
  /** Ctrl/Cmd + F —— 聚焦搜索框 */
  onSearch?: () => void;
  /** Ctrl/Cmd + Z —— 撤销 */
  onUndo?: () => void;
  /** Delete —— 删除选中（实现方须二次确认） */
  onDelete?: () => void;
  /** F5 —— 刷新 */
  onRefresh?: () => void;
}

interface ShortcutsRegistry {
  /** 注册一组处理器，返回注销函数（供 useEffect 清理） */
  register: (handlers: ShortcutHandlers) => () => void;
}

const ShortcutsContext = createContext<ShortcutsRegistry | null>(null);

/**
 * 判定事件目标是否为「可编辑元素」。
 * 命中则快捷键**不生效、也不 preventDefault**（把按键完全交还用户/浏览器），
 * 避免在输入框内按 Ctrl+F / Delete / Ctrl+Z 被全局拦截。
 */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (target.isContentEditable) return true; // 含祖先 contenteditable 的继承命中
  return target.closest('[contenteditable="true"]') !== null;
}

/**
 * 全局快捷键 Provider（N-12）。
 * - **仅挂一个 `window` keydown 监听**。
 * - 页面通过 `useRegisterShortcuts` 注册自己的处理器；从**最近注册者向最早**查找，
 *   命中第一个定义了对应回调的处理器才执行并 `preventDefault`；**未定义则不拦截**。
 * - 本文件不含 JSX（用 `createElement` 渲染 Provider），以保持设计约定的 `.ts` 文件名。
 */
export function GlobalShortcutsProvider({ children }: { children: ReactNode }) {
  const stackRef = useRef<ShortcutHandlers[]>([]);

  const register = useCallback((handlers: ShortcutHandlers) => {
    stackRef.current.push(handlers);
    return () => {
      stackRef.current = stackRef.current.filter(h => h !== handlers);
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // 输入框/文本域/下拉/contenteditable 内 → 完全不拦截
      if (isEditableTarget(e.target)) return;

      const ctrl = e.ctrlKey || e.metaKey;
      const key = e.key;

      // 从最近注册者向最早查找第一个提供该回调的处理器
      const pick = <K extends keyof ShortcutHandlers>(name: K): ShortcutHandlers[K] | undefined => {
        for (let i = stackRef.current.length - 1; i >= 0; i--) {
          const fn = stackRef.current[i][name];
          if (fn) return fn;
        }
        return undefined;
      };

      const trigger = (fn: (() => void) | undefined) => {
        if (!fn) return false; // 无处理器 → 不拦截、不 preventDefault
        e.preventDefault();
        fn();
        return true;
      };

      if (ctrl && (key === 'n' || key === 'N')) {
        trigger(pick('onNew'));
      } else if (ctrl && (key === 'f' || key === 'F')) {
        trigger(pick('onSearch'));
      } else if (ctrl && (key === 'z' || key === 'Z')) {
        trigger(pick('onUndo'));
      } else if (!ctrl && !e.altKey && key === 'Delete') {
        trigger(pick('onDelete'));
      } else if (key === 'F5') {
        trigger(pick('onRefresh'));
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  const value = useMemo<ShortcutsRegistry>(() => ({ register }), [register]);

  return createElement(ShortcutsContext.Provider, { value }, children);
}

/**
 * 页面注册快捷键。挂载时注册、卸载时注销；处理器经 ref 读取**最新**闭包，
 * 因此可安全传入每次渲染新建的回调而无需重注册。
 */
export function useRegisterShortcuts(handlers: ShortcutHandlers): void {
  const ctx = useContext(ShortcutsContext);
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  useEffect(() => {
    if (!ctx) return;
    // getter 代理：注册一次，取用时读取最新 handlers；未定义的回调返回 undefined（不拦截）
    const proxy: ShortcutHandlers = {
      get onNew() { return handlersRef.current.onNew; },
      get onSearch() { return handlersRef.current.onSearch; },
      get onUndo() { return handlersRef.current.onUndo; },
      get onDelete() { return handlersRef.current.onDelete; },
      get onRefresh() { return handlersRef.current.onRefresh; },
    };
    return ctx.register(proxy);
  }, [ctx]);
}
