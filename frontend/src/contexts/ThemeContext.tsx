import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import type { ThemePreset } from '../types';
import { isThemePreset } from '../themes/presets';

interface ThemeContextValue {
  /** 当前主题预设（暗色 / 亮色） */
  theme: ThemePreset;
  /** 显式切换到某个预设 */
  setTheme: (preset: ThemePreset) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

const STORAGE_KEY = 'rackviz-theme';

/**
 * 旧存储值 → 新预设的迁移映射（主题由 4 套精简为 2 套后，历史值不能失效）。
 * `dark`（旧暗色）→ `night`（夜间蓝/暗色）；`light`（旧亮色）→ `eye`（护眼绿/亮色）。
 */
const LEGACY_THEME_MAP: Record<string, ThemePreset> = {
  dark: 'night',
  light: 'eye',
};

/**
 * 读取持久化主题：
 * 1. 合法的新预设值（`night` / `eye`）直接采用；
 * 2. 旧值（`dark` / `light`）按 {@link LEGACY_THEME_MAP} 迁移；
 * 3. 其它非法/缺失值回落默认 `'night'`（暗色）。
 * @returns 校验/迁移后的主题预设
 */
function getInitialTheme(): ThemePreset {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isThemePreset(stored)) return stored;
    if (stored && stored in LEGACY_THEME_MAP) return LEGACY_THEME_MAP[stored];
  } catch {
    // localStorage 不可用（如隐私模式）时静默回落默认主题
  }
  return 'night';
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<ThemePreset>(getInitialTheme);

  // 同步 `data-theme` 属性（驱动 global.css 的 CSS 变量）并持久化
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    try {
      localStorage.setItem(STORAGE_KEY, theme);
    } catch {
      // 持久化失败不影响当前会话
    }
  }, [theme]);

  // 只接受合法预设值：非法输入被忽略，保持当前主题
  const setTheme = useCallback((preset: ThemePreset) => {
    if (!isThemePreset(preset)) return;
    setThemeState(preset);
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within ThemeProvider');
  return ctx;
}
