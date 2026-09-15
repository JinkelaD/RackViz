import { theme as antdTheme, type ThemeConfig } from 'antd';
import type { ThemePreset } from '../types';

/**
 * 2 套主题预设（N-15，已按用户要求精简）。单一来源：`main.tsx` 的 ConfigProvider
 * 与 `Layout.tsx` 的选择入口都从这里取。
 *
 * | key     | 展示名 | 角色   | antd 算法        |
 * |---------|--------|--------|------------------|
 * | `night` | 暗色   | 默认   | darkAlgorithm    |
 * | `eye`   | 亮色   | 备选   | defaultAlgorithm |
 *
 * 双轨同步约定（设计 §8-10）：
 * - **antd 组件**由本文件 `token` / `table` / `modal` 驱动（ConfigProvider）。
 * - **自定义 CSS** 由 `styles/global.css` 的 `[data-theme="<key>"]` 颜色 token 块驱动。
 * 两者的取值必须**保持同一色系**，新增/改动主题时两处同步。
 */

/** antd 语义 token 子集 */
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

/** 2 套预设（暗色 / 亮色） */
export const THEME_PRESETS: Record<ThemePreset, ThemePresetConfig> = {
  // 夜间蓝：深海军蓝 **暗色**（darkAlgorithm），浅蓝白文字保证高对比 —— 默认主题
  night: {
    key: 'night',
    label: '暗色',
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

  // 护眼绿：浅绿纸感 **亮色**（defaultAlgorithm），深绿灰文字保证高对比 —— 备选主题
  eye: {
    key: 'eye',
    label: '亮色',
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
};

/** 选择入口的展示顺序（默认暗色在前） */
export const THEME_PRESET_ORDER: ThemePreset[] = ['night', 'eye'];

/** 类型守卫：字符串 → ThemePreset（持久化读回时校验） */
export function isThemePreset(v: unknown): v is ThemePreset {
  return v === 'night' || v === 'eye';
}

/** 取某预设对应的 antd 算法函数 */
export function getAntdAlgorithm(preset: ThemePreset): ThemeConfig['algorithm'] {
  return THEME_PRESETS[preset].algorithm === 'dark'
    ? antdTheme.darkAlgorithm
    : antdTheme.defaultAlgorithm;
}
