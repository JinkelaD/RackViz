import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { ConfigProvider, theme, App as AntdApp } from 'antd';
import { ThemeProvider, useTheme } from './contexts/ThemeContext';
import App from './App';
import './styles/global.css';

const darkToken = {
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
};

const lightToken = {
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
};

const darkModal = {
  contentBg: '#1A1E2C',
  headerBg: '#1A1E2C',
  titleColor: '#EAECF2',
  titleFontSize: 15,
  colorText: '#8B92A0',
  colorIcon: '#8B92A0',
};

const lightModal = {
  contentBg: '#FFFFFF',
  headerBg: '#FFFFFF',
  titleColor: '#111827',
  titleFontSize: 15,
  colorText: '#6B7280',
  colorIcon: '#6B7280',
};

function AntdThemeWrapper({ children }: { children: React.ReactNode }) {
  const { theme: current } = useTheme();
  const isDark = current === 'dark';

  return (
    <ConfigProvider
      theme={{
        algorithm: isDark ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: {
          fontFamily: "'Inter', 'PingFang SC', 'Microsoft YaHei', system-ui, -apple-system, sans-serif",
          borderRadius: 6,
          ...(isDark ? darkToken : lightToken),
        },
        components: {
          Modal: isDark ? darkModal : lightModal,
          Table: {
            headerBg: isDark ? '#1A1E2C' : '#F3F4F6',
            headerColor: isDark ? '#8B92A0' : '#6B7280',
            rowHoverBg: isDark ? '#222738' : '#E5E7EB',
            borderColor: isDark ? '#2A2E3C' : '#D1D5DB',
          },
          Button: {
            primaryShadow: 'none',
            defaultShadow: 'none',
            dangerShadow: 'none',
          },
        },
      }}
    >
      <AntdApp>
        {children}
      </AntdApp>
    </ConfigProvider>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter>
      <ThemeProvider>
        <AntdThemeWrapper>
          <App />
        </AntdThemeWrapper>
      </ThemeProvider>
    </BrowserRouter>
  </React.StrictMode>
);
