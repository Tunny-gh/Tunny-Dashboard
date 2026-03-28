import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  // 【JSX変換設定】: Vite 8 / OXC 環境でテスト時に自動 JSX ランタイムを有効にする
  esbuild: {
    jsx: 'automatic',
    jsxImportSource: 'react',
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  test: {
    // 【テスト環境】: React コンポーネントテストのために jsdom を使用
    environment: 'jsdom',
    // 【セットアップファイル】: @testing-library/jest-dom の matcher を有効化
    setupFiles: ['./src/test/setup.ts'],
    // 【グローバル】: describe/test/expect を import なしで使用可能
    globals: true,
    // 【除外パターン】: Playwright E2E テストファイルは Vitest の対象外
    exclude: ['**/node_modules/**', '**/dist/**', 'e2e/**'],
  },
});
