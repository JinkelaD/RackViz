import { useCallback, useEffect, useRef, useState } from 'react';
import type { Device, DeviceQuery } from '../types';
import { queryDevices } from '../tauri-api';

/** 搜索词防抖窗口（避免每键一次 IPC） */
const SEARCH_DEBOUNCE_MS = 300;
const DEFAULT_LIMIT = 10;

export interface UsePagedDevicesReturn {
  /** 当前页数据 */
  items: Device[];
  /** 同 WHERE 条件下总数（服务端 COUNT） */
  total: number;
  loading: boolean;
  /** 当前「已提交」查询（含即时 search 原文，供受控输入框使用） */
  query: DeviceQuery;
  /** 部分更新查询（合并而非覆盖）；切 search / room_id 时自动重置 offset=0 */
  setQuery: (patch: Partial<DeviceQuery>) => void;
  /** 用最新查询重取当前页（删除 / 批量删除 / 导入 / 恢复后调用） */
  refresh: () => void;
}

/**
 * 服务端分页 + 多字段搜索 Hook（N-01/N-02）。
 *
 * 关键机制：
 * - **搜索防抖**：仅对 `search` 施加 ~300ms 防抖；offset/limit/room_id/sort 变化立即取数。
 * - **防竞态**：每次请求递增 `seqRef` 序号；响应返回时若序号已过期（有更新的请求发出）则丢弃，
 *   避免快速输入/切页时旧响应后到覆盖新数据（显示错页）。
 * - **部分更新**：`setQuery` 合并补丁；补丁含 `search`/`room_id` 时把 `offset` 归零到第一页。
 */
export function usePagedDevices(initial?: Partial<DeviceQuery>): UsePagedDevicesReturn {
  const [query, setQueryState] = useState<DeviceQuery>(() => ({
    search: null,
    room_id: null,
    rack_id: null,
    include_deleted: null,
    sort_field: null,
    sort_order: null,
    offset: 0,
    limit: DEFAULT_LIMIT,
    ...initial,
  }));

  // 已提交查询中的 search 原文用于输入框；实际取数用防抖后的 debouncedSearch
  const [debouncedSearch, setDebouncedSearch] = useState<string | null>(query.search ?? null);
  const [items, setItems] = useState<Device[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);

  // 请求序号：只有最新序号的响应才允许落地
  const seqRef = useRef(0);

  const runFetch = useCallback((q: DeviceQuery) => {
    const seq = ++seqRef.current;
    setLoading(true);
    queryDevices(q)
      .then((page) => {
        if (seq !== seqRef.current) return; // 过期响应丢弃（防竞态）
        setItems(page.items);
        setTotal(page.total);
      })
      .catch((err) => {
        if (seq !== seqRef.current) return;
        console.error('加载设备分页失败:', err);
        setItems([]);
        setTotal(0);
      })
      .finally(() => {
        if (seq === seqRef.current) setLoading(false);
      });
  }, []);

  const setQuery = useCallback((patch: Partial<DeviceQuery>) => {
    setQueryState((prev) => {
      const next: DeviceQuery = { ...prev, ...patch };
      if ('search' in patch || 'room_id' in patch) {
        next.offset = 0; // 切搜索词 / 切机房 → 回到第一页
      }
      return next;
    });
  }, []);

  // 搜索防抖：query.search 变化后延迟提交
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(query.search ?? null), SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [query.search]);

  // 始终保存「最新有效查询」，供 refresh 使用（避免 refresh 依赖项随查询变化而重建）
  const effectiveQuery: DeviceQuery = { ...query, search: debouncedSearch };
  const latestQueryRef = useRef(effectiveQuery);
  latestQueryRef.current = effectiveQuery;

  // 取数：搜索用防抖值，其余字段即时
  useEffect(() => {
    runFetch({
      search: debouncedSearch,
      room_id: query.room_id ?? null,
      rack_id: query.rack_id ?? null,
      include_deleted: query.include_deleted ?? null,
      sort_field: query.sort_field ?? null,
      sort_order: query.sort_order ?? null,
      offset: query.offset ?? 0,
      limit: query.limit ?? DEFAULT_LIMIT,
    });
  }, [
    debouncedSearch,
    query.room_id,
    query.rack_id,
    query.sort_field,
    query.sort_order,
    query.offset,
    query.limit,
    runFetch,
  ]);

  const refresh = useCallback(() => {
    runFetch(latestQueryRef.current);
  }, [runFetch]);

  return { items, total, loading, query, setQuery, refresh };
}
