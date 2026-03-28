/**
 * Playwright E2E テスト設定 (TASK-1502)
 *
 * 【役割】: Tunny Dashboard E2E テストの設定ファイル
 * 【設計方針】:
 *   - webServer で Vite dev サーバーを自動起動
 *   - CI 環境では COOP/COEP ヘッダー確認を含む
 *   - ブラウザ: Chromium のみ（SharedArrayBuffer サポートに必要）
 * 🟢 NFR-010: 単一 HTML ファイル動作確認
 * 🟢 REQ-001: Cross-Origin ヘッダー設定確認
 *
 * ## 実行前の準備
 * ```bash
 * # ブラウザバイナリのインストール（初回のみ）
 * npx playwright install chromium
 *
 * # E2E テスト実行
 * npm run test:e2e
 * ```
 */

import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  /** 【テストディレクトリ】: E2E テストファイルの場所 */
  testDir: './e2e',

  /** 【タイムアウト】: 1テストあたり最大 30 秒 */
  timeout: 30_000,

  /** 【グローバルタイムアウト】: 全スイートで最大 5 分 */
  globalTimeout: 300_000,

  /** 【失敗時の再試行】: CI では 1 回リトライ、ローカルでは再試行しない */
  retries: process.env.CI ? 1 : 0,

  /** 【並列実行】: ファイル単位で並列実行 */
  fullyParallel: true,

  /** 【レポート形式】: CI では GitHub Actions 対応レポート、ローカルは HTML */
  reporter: process.env.CI ? 'github' : 'html',

  use: {
    /** 【ベース URL】: Vite dev サーバーのデフォルトポート */
    baseURL: 'http://localhost:5173',

    /** 【ヘッドレス】: 常にヘッドレスモードで実行 */
    headless: true,

    /** 【スクリーンショット】: 失敗時のみ取得 */
    screenshot: 'only-on-failure',

    /** 【トレース】: 失敗時のみ収集 */
    trace: 'on-first-retry',
  },

  /** 【プロジェクト設定】: Chromium のみ（COOP/COEP + SharedArrayBuffer 対応） */
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // 【SharedArrayBuffer 有効化】: COOP/COEP が設定されているため有効になる
        launchOptions: {
          args: ['--enable-features=SharedArrayBuffer'],
        },
      },
    },
  ],

  /** 【Web サーバー設定】: テスト実行前に Vite dev サーバーを起動 */
  webServer: {
    /** 【起動コマンド】: npm run dev で Vite 開発サーバーを起動 */
    command: 'npm run dev',
    /** 【ポート】: Vite のデフォルトポート */
    port: 5173,
    /**
     * 【既存サーバー再利用】:
     * - ローカル開発時: 既存サーバーを再利用（高速化）
     * - CI 環境: 毎回新規起動（環境の一貫性確保）
     */
    reuseExistingServer: !process.env.CI,
    /** 【起動タイムアウト】: dev サーバー起動に最大 30 秒 */
    timeout: 30_000,
  },
});
