import { useState, useEffect, useCallback, useRef } from 'react';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyRecord = Record<string, any>;

interface UseApiListOptions {
  optimistic?: boolean;
}

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
    const prev = items;
    setItems(prevItems => prevItems.map(d => d.id === id ? { ...d, ...data } as T : d));
    try {
      await updateFnRef.current(id, data);
      refresh();
    } catch {
      setItems(prev);
      refresh();
    }
  }, [refresh, items, optimistic]);

  const remove = useCallback(async (id: number) => {
    if (!optimistic) {
      await deleteFnRef.current(id);
      refresh();
      return;
    }
    const prev = items;
    setItems(prevItems => prevItems.filter(d => d.id !== id));
    try {
      await deleteFnRef.current(id);
      refresh();
    } catch {
      setItems(prev);
      refresh();
    }
  }, [refresh, items, optimistic]);

  return { items, loading, refresh, create, update, remove };
}
