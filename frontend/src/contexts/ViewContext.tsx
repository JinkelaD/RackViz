import { createContext, useContext, useMemo, useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import type { ViewMode } from '../types';

/** 机柜视图共享状态（Layout 级 Provider，字段 <10） */
export interface ViewContextValue {
  view: ViewMode;
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
  searchQuery: string;
  onSearchChange: (q: string) => void;
  showAddRack: boolean;
  setShowAddRack: (v: boolean) => void;
}

export const ViewContext = createContext<ViewContextValue | undefined>(undefined);

export function useViewContext(): ViewContextValue {
  const value = useContext(ViewContext);
  if (!value) {
    throw new Error('useViewContext 必须在 <ViewProvider> 内使用');
  }
  return value;
}

/** 提供视图/缩放/搜索/添加机柜弹窗状态（仅机柜页消费） */
export function ViewProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<ViewMode>('front');
  const [zoom, setZoom] = useState(100);
  const [searchQuery, setSearchQuery] = useState('');
  const [showAddRack, setShowAddRack] = useState(false);

  const onZoomIn = useCallback(() => setZoom(z => Math.min(z + 10, 200)), []);
  const onZoomOut = useCallback(() => setZoom(z => Math.max(z - 10, 50)), []);
  const onZoomReset = useCallback(() => setZoom(100), []);

  const value = useMemo<ViewContextValue>(() => ({
    view, zoom, onZoomIn, onZoomOut, onZoomReset,
    searchQuery, onSearchChange: setSearchQuery,
    showAddRack, setShowAddRack,
  }), [view, zoom, onZoomIn, onZoomOut, onZoomReset, searchQuery, showAddRack]);

  return <ViewContext.Provider value={value}>{children}</ViewContext.Provider>;
}
