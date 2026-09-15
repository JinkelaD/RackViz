import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { ConfigProvider, App as AntdApp } from 'antd';
import { ThemeProvider, useTheme } from './contexts/ThemeContext';
import { THEME_PRESETS, getAntdAlgorithm } from './themes/presets';
import App from './App';
import './styles/global.css';

function AntdThemeWrapper({ children }: { children: React.ReactNode }) {
  const { theme: current } = useTheme();
  const preset = THEME_PRESETS[current];

  return (
    <ConfigProvider
      theme={{
        algorithm: getAntdAlgorithm(current),
        token: {
          fontFamily: "'Inter', 'PingFang SC', 'Microsoft YaHei', system-ui, -apple-system, sans-serif",
          borderRadius: 6,
          ...preset.token,
        },
        components: {
          Modal: preset.modal,
          Table: preset.table,
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
