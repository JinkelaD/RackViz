import { theme as antdTheme, type ThemeConfig } from 'antd';
import type { ThemePreset } from '../types';

/**
 * 4 套主题预设（N-15）。单一来源：`main.tsx` 的 ConfigProvider 与 `Layout.tsx` 的选择入口都从这里取。
 *
 * 双轨同步约定（设计 §8-10）：
 * - **antd 组件**由本文件 `token` / `table` / `modal` 驱动（ConfigProvider）。
 * - **自定义 CSS** 由 `styles/global.css` 的 `[data-theme="<key>"]` 颜色 token 块驱动。
 * 两者的取值必须**保持同一色系**，新增/改动主题时两处同步。
 */

/** antd 语义 token 子集（对齐现有 dark/light 取值，避免全站回归） */
export interface ThemeTokens {
  colorPrimary: string;
  colorSuccess: string;
  colorWarning: string;
  colorError: string;
  colorInfo: string;
  colorBgContainer: string;
  colorBgElevated: string;
  colorBgLayout: string;
  colorBorder: string;
  colorBorderSecondary: string;
  colorTextBase: string;
  colorText: string;
  colorTextSecondary: string;
  colorBgTextHover: string;
  colorBgTextActive: string;
}

/** antd Table 组件级 token（表头/悬浮行/描边） */
export interface ThemeTableTokens {
  headerBg: string;
  headerColor: string;
  rowHoverBg: string;
  borderColor: string;
}

/** antd Modal 组件级 token */
export interface ThemeModalTokens {
  contentBg: string;
  headerBg: string;
  titleColor: string;
  titleFontSize: number;
  colorText: string;
  colorIcon: string;
}

export interface ThemePresetConfig {
  /** 与 `data-theme` 属性值、`ThemePreset` 联合类型一致 */
  key: ThemePreset;
  /** UI 展示名 */
  label: string;
  /** antd 算法取向：暗色系 → darkAlgorithm，亮色系 → defaultAlgorithm */
  algorithm: 'dark' | 'light';
  token: ThemeTokens;
  table: ThemeTableTokens;
  modal: ThemeModalTokens;
}

/** 4 套预设（暗 / 亮 / 护眼绿 / 夜间蓝） */
export const THEME_PRESETS: Record<ThemePreset, ThemePresetConfig> = {
  dark: {
    key: 'dark',
    label: '暗色（默认）',
    algorithm: 'dark',
    token: {
      colorBgContainer: '#1A1E2C',
      colorBgElevated: '#1A1E2C',
      colorBgLayout: '#0F1117',
      colorBorder: '#2A2E3C',
      colorBorderSecondary: '#2A2E3C',
      colorPrimary: '#4B6BFB',
      colorSuccess: '#10B981',
      colorWarning: '#F59E0B',
      colorError: '#EF4444',
      colorInfo: '#06B6D4',
      colorTextBase: '#EAECF2',
      colorText: '#EAECF2',
      colorTextSecondary: '#8B92A0',
      colorBgTextHover: '#222738',
      colorBgTextActive: '#1A1E2C',
    },
    table: {
      headerBg: '#1A1E2C',
      headerColor: '#8B92A0',
      rowHoverBg: '#222738',
      borderColor: '#2A2E3C',
    },
    modal: {
      contentBg: '#1A1E2C',
      headerBg: '#1A1E2C',
      titleColor: '#EAECF2',
      titleFontSize: 15,
      colorText: '#8B92A0',
      colorIcon: '#8B92A0',
    },
  },

  light: {
    key: 'light',
    label: '亮色',
    algorithm: 'light',
    token: {
      colorBgContainer: '#FFFFFF',
      colorBgElevated: '#FFFFFF',
      colorBgLayout: '#F3F4F6',
      colorBorder: '#D1D5DB',
      colorBorderSecondary: '#D1D5DB',
      colorPrimary: '#4B6BFB',
      colorSuccess: '#10B981',
      colorWarning: '#F59E0B',
      colorError: '#EF4444',
      colorInfo: '#0891B2',
      colorTextBase: '#111827',
      colorText: '#111827',
      colorTextSecondary: '#6B7280',
      colorBgTextHover: '#E5E7EB',
      colorBgTextActive: '#F3F4F6',
    },
    table: {
      headerBg: '#F3F4F6',
      headerColor: '#6B7280',
      rowHoverBg: '#E5E7EB',
      borderColor: '#D1D5DB',
    },
    modal: {
      contentBg: '#FFFFFF',
      headerBg: '#FFFFFF',
      titleColor: '#111827',
      titleFontSize: 15,
      colorText: '#6B7280',
      colorIcon: '#6B7280',
    },
  },

  // 护眼绿：浅绿纸感 **亮色**（defaultAlgorithm），深绿灰文字保证高对比
  eye: {
    key: 'eye',
    label: '护眼绿',
    algorithm: 'light',
    token: {
      colorBgContainer: '#FFFFFF',
      colorBgElevated: '#FFFFFF',
      colorBgLayout: '#EEF5EA',
      colorBorder: '#C5D8BC',
      colorBorderSecondary: '#C5D8BC',
      colorPrimary: '#2E7D32',
      colorSuccess: '#2E7D32',
      colorWarning: '#B45309',
      colorError: '#C0392B',
      colorInfo: '#0F766E',
      colorTextBase: '#1B2E1B',
      colorText: '#1B2E1B',
      colorTextSecondary: '#4A5D48',
      colorBgTextHover: '#DCE8D6',
      colorBgTextActive: '#EEF5EA',
    },
    table: {
      headerBg: '#E4EFDD',
      headerColor: '#4A5D48',
      rowHoverBg: '#DCE8D6',
      borderColor: '#C5D8BC',
    },
    modal: {
      contentBg: '#FFFFFF',
      headerBg: '#FFFFFF',
      titleColor: '#1B2E1B',
      titleFontSize: 15,
      colorText: '#4A5D48',
      colorIcon: '#4A5D48',
    },
  },

  // 夜间蓝：深海军蓝 **暗色**（darkAlgorithm），浅蓝白文字保证高对比
  night: {
    key: 'night',
    label: '夜间蓝',
    algorithm: 'dark',
    token: {
      colorBgContainer: '#172A4A',
      colorBgElevated: '#172A4A',
      colorBgLayout: '#0E1930',
      colorBorder: '#24395E',
      colorBorderSecondary: '#24395E',
      colorPrimary: '#5B8DEF',
      colorSuccess: '#34D399',
      colorWarning: '#FBBF24',
      colorError: '#F87171',
      colorInfo: '#22D3EE',
      colorTextBase: '#E6EDFA',
      colorText: '#E6EDFA',
      colorTextSecondary: '#9DB0D0',
      colorBgTextHover: '#1F3559',
      colorBgTextActive: '#172A4A',
    },
    table: {
      headerBg: '#172A4A',
      headerColor: '#9DB0D0',
      rowHoverBg: '#1F3559',
      borderColor: '#24395E',
    },
    modal: {
      contentBg: '#172A4A',
      headerBg: '#172A4A',
      titleColor: '#E6EDFA',
      titleFontSize: 15,
      colorText: '#9DB0D0',
      colorIcon: '#9DB0D0',
    },
  },
};

/** 选择入口的展示顺序 */
export const THEME_PRESET_ORDER: ThemePreset[] = ['dark', 'light', 'eye', 'night'];

/** 类型守卫：字符串 → ThemePreset（持久化读回时校验） */
export function isThemePreset(v: unknown): v is ThemePreset {
  return v === 'dark' || v === 'light' || v === 'eye' || v === 'night';
}

/** 取某预设对应的 antd 算法函数 */
export function getAntdAlgorithm(preset: ThemePreset): ThemeConfig['algorithm'] {
  return THEME_PRESETS[preset].algorithm === 'dark'
    ? antdTheme.darkAlgorithm
    : antdTheme.defaultAlgorithm;
}
