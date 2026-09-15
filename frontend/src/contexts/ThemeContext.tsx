import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import type { ThemePreset } from '../types';
import { isThemePreset } from '../themes/presets';

interface ThemeContextValue {
  /** 当前主题预设（暗 / 亮 / 护眼绿 / 夜间蓝） */
  theme: ThemePreset;
  /** 显式切换到某个预设；4 主题下"翻转"语义已无意义，故用 setTheme */
  setTheme: (preset: ThemePreset) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

const STORAGE_KEY = 'rackviz-theme';

/**
 * 读取持久化主题。任意非法/缺失值一律回落 `'dark'`（默认主题）。
 * @returns 校验通过的主题预设
 */
function getInitialTheme(): ThemePreset {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isThemePreset(stored)) return stored;
  } catch {
    // localStorage 不可用（如隐私模式）时静默回落默认主题
  }
  return 'dark';
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

  // 只接受 4 个合法值：非法输入被忽略，保持当前主题
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
