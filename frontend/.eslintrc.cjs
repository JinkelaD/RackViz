/**
 * RackViz 前端 ESLint 配置（v1.2）
 * 安装依赖后生效：npm i -D eslint @typescript-eslint/parser @typescript-eslint/eslint-plugin eslint-plugin-react-hooks
 * 离线环境请使用 `npm run lint:offline`（scripts/lint-check.mjs，零依赖）
 *
 * ⚠️ 兼容状态（2026-09-16 实测）：typescript-eslint@8 官方支持 TS <6.1.0，
 *    本项目 TS 7.0.2 触发硬门禁报错 "typescript-eslint does not support TS 7.0"
 *    （上游 issue #10940 跟踪 TS >=7.1 支持）。依赖已回滚，
 *    `npm run lint` 暂指向离线红线检查 scripts/lint-check.mjs。
 *    上游发布兼容版本后：安装上述依赖并把 package.json 的 lint script 改回
 *    "eslint src/ --ext .ts,.tsx" 即可，本配置无需改动。
 */
module.exports = {
  root: true,
  env: { browser: true, es2022: true },
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaVersion: 2022,
    sourceType: 'module',
    ecmaFeatures: { jsx: true },
    project: './tsconfig.json',
  },
  plugins: ['@typescript-eslint', 'react-hooks'],
  extends: ['eslint:recommended', 'plugin:@typescript-eslint/recommended'],
  ignorePatterns: ['dist/', 'node_modules/', '*.tsbuildinfo'],
  rules: {
    // 审查规范 §8.2
    '@typescript-eslint/no-explicit-any': 'error',
    '@typescript-eslint/no-non-null-assertion': 'warn',
    '@typescript-eslint/no-unnecessary-type-assertion': 'warn',
    '@typescript-eslint/no-unsafe-assignment': 'warn',
    '@typescript-eslint/no-unsafe-member-access': 'warn',
    'no-console': ['warn', { allow: ['warn', 'error'] }],
    'react-hooks/exhaustive-deps': 'warn',
    'react-hooks/rules-of-hooks': 'error',
  },
};
