import type { UserConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default {
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome105', 'safari13'],
    // Vite 8 起使用 Oxc 压缩（esbuild 已弃用且不再随 Vite 安装）
    minify: !process.env.TAURI_DEBUG ? 'oxc' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    // Vite 8 改用 Rolldown：`rollupOptions.output.manualChunks` 的「对象形式」已不再支持，
    // 语义等价的迁移目标为 `rolldownOptions.output.codeSplitting.groups`（vendor / antd 分包不变）。
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: 'vendor',
              test: /[\\/]node_modules[\\/](react|react-dom|react-router|react-router-dom|scheduler)[\\/]/,
              priority: 20,
            },
            {
              name: 'antd',
              test: /[\\/]node_modules[\\/](antd|@ant-design)[\\/]/,
              priority: 10,
            },
          ],
        },
      },
    },
    chunkSizeWarningLimit: 800,
  },
} satisfies UserConfig;
