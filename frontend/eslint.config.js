import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';

export default tseslint.config(
  { ignores: ['dist', '.tauri', 'node_modules'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
      // 红线规则：禁止 any 类型
      '@typescript-eslint/no-explicit-any': 'error',
      // 红线规则：禁止非空断言
      '@typescript-eslint/no-non-null-assertion': 'error',
      // 警告：console.log/console.error 应改为用户可见提示
      'no-console': ['warn', { allow: ['warn', 'error'] }],
    },
  },
);
