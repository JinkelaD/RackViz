import { useState, useEffect, useCallback, useRef } from 'react';

interface UseApiListOptions {
  optimistic?: boolean;
}

/**
 * 通用 CRUD Hook：
 * - list/create/update/delete 函数通过 ref 稳定持有（useCallback 空依赖，杜绝无限刷新）
 * - 乐观更新：函数式 setState（并发请求不互相覆盖）；失败回滚用 itemsRef 最新快照（而非过期闭包）
 */
export function useApiList<T extends { id: number }>(
  listFn: () => Promise<T[]>,
  createFn: (data: Partial<T>) => Promise<T>,
  updateFn: (id: number, data: Partial<T>) => Promise<T | null>,
  deleteFn: (id: number) => Promise<boolean>,
  options?: UseApiListOptions,
) {
  const [items, setItems] = useState<T[]>([]);
  const [loading, setLoading] = useState(true);
  const optimistic = options?.optimistic ?? false;

  // 用 ref 稳定持有函数引用，避免 useCallback 依赖项变化导致无限刷新
  const listFnRef = useRef(listFn);
  const createFnRef = useRef(createFn);
  const updateFnRef = useRef(updateFn);
  const deleteFnRef = useRef(deleteFn);

  listFnRef.current = listFn;
  createFnRef.current = createFn;
  updateFnRef.current = updateFn;
  deleteFnRef.current = deleteFn;

  // 始终同步最新 items，供失败回滚使用（避免闭包捕获过期值）
  const itemsRef = useRef(items);
  itemsRef.current = items;

  const refresh = useCallback(() => {
    setLoading(true);
    listFnRef.current().then(setItems).catch(console.error).finally(() => setLoading(false));
  }, []); // 空依赖 — 永远稳定

  useEffect(() => { refresh(); }, [refresh]);

  const create = useCallback(async (data: Partial<T>) => {
    await createFnRef.current(data);
    refresh();
  }, [refresh]);

  const update = useCallback(async (id: number, data: Partial<T>) => {
    if (!optimistic) {
      await updateFnRef.current(id, data);
      refresh();
      return;
    }
    // 先本地乐观更新；失败回滚到 itemsRef 快照，再由 refresh 拉取权威数据
    const prev = itemsRef.current;
    setItems(prevItems => prevItems.map(d => d.id === id ? { ...d, ...data } as T : d));
    try {
      await updateFnRef.current(id, data);
      refresh();
    } catch (error) {
      console.error('更新失败，回滚:', error);
      setItems(prev);
      refresh();
    }
  }, [refresh, optimistic]);

  const remove = useCallback(async (id: number) => {
    if (!optimistic) {
      await deleteFnRef.current(id);
      refresh();
      return;
    }
    const prev = itemsRef.current;
    setItems(prevItems => prevItems.filter(d => d.id !== id));
    try {
      await deleteFnRef.current(id);
      refresh();
    } catch (error) {
      console.error('删除失败，回滚:', error);
      setItems(prev);
      refresh();
    }
  }, [refresh, optimistic]);

  return { items, loading, refresh, create, update, remove };
}
