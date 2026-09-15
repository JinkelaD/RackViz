import { useState, useEffect } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { App, Button, Dropdown, Space } from 'antd';
import { open } from '@tauri-apps/plugin-dialog';
import { useTheme } from '../contexts/ThemeContext';
import { THEME_PRESETS, THEME_PRESET_ORDER, isThemePreset } from '../themes/presets';
import { ViewProvider } from '../contexts/ViewContext';
import { RoomProvider } from '../contexts/RoomContext';
import * as tauriApi from '../tauri-api';

export default function Layout() {
  const navigate = useNavigate();
  const location = useLocation();
  const { modal, message } = App.useApp();

  // 设置弹窗 & 日志配置
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [loggingEnabled, setLoggingEnabled] = useState(false);
  const [logDir, setLogDir] = useState('');
  const [loggingLoading, setLoggingLoading] = useState(false);

  // N-18 备份 / 恢复
  const [backupLoading, setBackupLoading] = useState(false);
  const [restoreLoading, setRestoreLoading] = useState(false);

  const { theme: currentTheme, setTheme } = useTheme();

  // N-15：4 套主题预设的选择菜单（当前项带 ✓）
  const themeMenuItems = THEME_PRESET_ORDER.map(key => ({
    key,
    label: (
      <span className="theme-menu-item">
        <span className={`theme-swatch theme-swatch--${key}`} />
        {THEME_PRESETS[key].label}
        {currentTheme === key && <span className="theme-check">✓</span>}
      </span>
    ),
  }));

  // 打开设置时加载日志配置
  useEffect(() => {
    if (settingsOpen) {
      tauriApi.getLoggingConfig().then(cfg => {
        setLoggingEnabled(cfg.enabled);
        setLogDir(cfg.logDir);
      }).catch(console.error);
    }
  }, [settingsOpen]);

  const handleToggleLogging = async (checked: boolean) => {
    setLoggingLoading(true);
    try {
      const cfg = await tauriApi.setLoggingEnabled(checked);
      setLoggingEnabled(cfg.enabled);
      setLogDir(cfg.logDir);
    } catch (err) {
      console.error('切换日志失败:', err);
    } finally {
      setLoggingLoading(false);
    }
  };

  const handleOpenLogDir = async () => {
    try {
      await tauriApi.openLogDir();
    } catch (err) {
      console.error('打开日志目录失败:', err);
    }
  };

  // N-18：一键备份（Rust 侧弹原生保存对话框，返回备份文件路径）
  const handleBackup = async () => {
    setBackupLoading(true);
    try {
      const path = await tauriApi.backupDatabase();
      message.success(`备份完成：${path}`);
    } catch (err) {
      message.error(`备份失败：${tauriApi.errorMessage(err)}`);
    } finally {
      setBackupLoading(false);
    }
  };

  // N-18：从备份恢复（选文件 → 二次确认 → 延迟恢复 + 提示重启）
  const handleRestore = async () => {
    let path: string;
    try {
      const selected = await open({
        filters: [{ name: '数据库备份', extensions: ['db', 'sqlite', 'sqlite3'] }],
        multiple: false,
      });
      if (!selected) return;
      path = typeof selected === 'string' ? selected : String((selected as { path?: string }).path ?? '');
    } catch (err) {
      message.error(`选择备份文件失败：${tauriApi.errorMessage(err)}`);
      return;
    }

    modal.confirm({
      title: '确认从备份恢复',
      content: (
        <div>
          <p>恢复将<b>覆盖当前全部数据</b>，此操作不可撤销。</p>
          <p>恢复后需要重启应用才能生效，请先确认已保存当前工作。</p>
        </div>
      ),
      okText: '恢复',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        setRestoreLoading(true);
        try {
          const res = await tauriApi.restoreDatabase(path);
          if (res.restart_required) {
            modal.info({
              title: '恢复已准备，请重启应用',
              content: res.message || '数据恢复已完成准备，请关闭并重新打开应用以生效。',
              okText: '我知道了',
            });
          } else {
            message.success(res.message || '恢复完成');
          }
        } catch (err) {
          message.error(`恢复失败：${tauriApi.errorMessage(err)}`);
        } finally {
          setRestoreLoading(false);
        }
      },
    });
  };


  const navItems = [
    { path: '/racks', label: '机柜管理' },
    { path: '/devices', label: '设备台账' },
  ];

  return (
    <ViewProvider>
      <RoomProvider>
        <div id="app">
          <header id="header">
            <div className="logo">Rack<span>Viz</span></div>
            <span className="version">v1.2</span>
            <Dropdown
              menu={{
                items: themeMenuItems,
                onClick: ({ key }) => { if (isThemePreset(key)) setTheme(key); },
              }}
              trigger={['click']}
              placement="bottomRight"
            >
              <button className="header-btn" title="切换主题">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <circle cx="12" cy="12" r="10"/>
                  <circle cx="12" cy="9" r="1.6" fill="currentColor" stroke="none"/>
                  <circle cx="8.2" cy="12.5" r="1.6" fill="currentColor" stroke="none"/>
                  <circle cx="15.8" cy="12.5" r="1.6" fill="currentColor" stroke="none"/>
                  <circle cx="12" cy="16" r="1.6" fill="currentColor" stroke="none"/>
                </svg>
              </button>
            </Dropdown>
            <button className="header-btn" onClick={() => setSettingsOpen(true)} title="设置">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
              </svg>
            </button>
            <nav className="nav-tabs">
              {navItems.map(item => (
                <button
                  key={item.path}
                  className={`nav-tab ${location.pathname === item.path ? 'active' : ''}`}
                  onClick={() => navigate(item.path)}
                >
                  {item.path === '/racks' && (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="8" x2="22" y2="8"/><line x1="2" y1="16" x2="22" y2="16"/><line x1="8" y1="2" x2="8" y2="22"/>
                    </svg>
                  )}
                  {item.path === '/devices' && (
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="6" x2="22" y2="6"/><line x1="2" y1="10" x2="22" y2="10"/><line x1="2" y1="14" x2="22" y2="14"/><line x1="2" y1="18" x2="22" y2="18"/>
                    </svg>
                  )}
                  {item.label}
                </button>
              ))}
            </nav>
          </header>

          <main id="main">
            <Outlet />
          </main>

          {/* 设置弹窗 */}
          {settingsOpen && (
            <div className="settings-overlay" onClick={() => setSettingsOpen(false)}>
              <div className="settings-modal" onClick={e => e.stopPropagation()}>
                <div className="settings-header">
                  <h3>设置</h3>
                  <button className="settings-close" onClick={() => setSettingsOpen(false)}>✕</button>
                </div>
                <div className="settings-body">
                  <div className="settings-section">
                    <div className="settings-section-title">
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>
                      </svg>
                      日志收集
                    </div>
                    <p className="settings-desc">
                      开启后，系统运行日志将写入本地文件，便于问题排查和 Bug 收集。
                      日志文件按天轮转，最多保留 7 天。
                    </p>
                    <div className="settings-row">
                      <label className="settings-switch">
                        <input
                          type="checkbox"
                          checked={loggingEnabled}
                          onChange={e => handleToggleLogging(e.target.checked)}
                          disabled={loggingLoading}
                        />
                        <span className="settings-slider" />
                      </label>
                      <span className="settings-label">{loggingEnabled ? '已开启' : '已关闭'}</span>
                    </div>
                    {logDir && (
                      <div className="settings-logdir">
                        <span className="settings-logdir-path" title={logDir}>{logDir}</span>
                        <button className="settings-logdir-btn" onClick={handleOpenLogDir} title="打开日志目录">
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>
                          </svg>
                        </button>
                      </div>
                    )}
                  </div>

                  <div className="settings-section">
                    <div className="settings-section-title">
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
                      </svg>
                      数据备份与恢复
                    </div>
                    <p className="settings-desc">
                      备份将导出当前数据库的一致性快照文件；从备份恢复会覆盖当前全部数据，恢复后需重启应用生效。
                    </p>
                    <Space>
                      <Button onClick={handleBackup} loading={backupLoading} disabled={restoreLoading}>
                        备份数据库
                      </Button>
                      <Button danger onClick={handleRestore} loading={restoreLoading} disabled={backupLoading}>
                        从备份恢复
                      </Button>
                    </Space>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>
      </RoomProvider>
    </ViewProvider>
  );
}
