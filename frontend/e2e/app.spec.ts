/**
 * E2E テスト: アプリ起動・セキュリティヘッダー確認 (TASK-1502)
 *
 * 【テスト対象】: Tunny Dashboard アプリケーション全体
 * 【テスト方針】:
 *   - アプリが正常に読み込まれることを確認
 *   - SharedArrayBuffer 使用に必要な COOP/COEP ヘッダーが設定されていることを確認
 *   - 単一 HTML ファイル配布の基盤となるビルド構成を確認
 * 🟢 REQ-001: Cross-Origin-Opener-Policy: same-origin
 * 🟢 REQ-002: Cross-Origin-Embedder-Policy: require-corp
 * 🟢 NFR-010: Python 環境不要の単一 HTML ファイル動作
 */

import { test, expect } from '@playwright/test';

// -------------------------------------------------------------------------
// TC-1502-E01: アプリ正常読み込み
// -------------------------------------------------------------------------

test('TC-1502-E01: アプリが正常に読み込まれてHTTP 200を返す', async ({ page }) => {
  // 【テスト目的】: アプリが正常に起動していることを確認 🟢
  // 【テスト内容】: トップページへのアクセスが 200 OK であること
  // 【期待される動作】: ページが正常にロードされる

  const response = await page.goto('/');

  // 【結果検証】: HTTP ステータスコードが 200 であること
  expect(response?.status()).toBe(200); // 【確認内容】: アプリが正常に起動している
});

test('TC-1502-E02: ページタイトルが設定されている', async ({ page }) => {
  // 【テスト目的】: ページタイトルが正しく設定されていることを確認 🟢
  // 【テスト内容】: HTML の <title> 要素の内容を確認

  await page.goto('/');

  // 【結果検証】: タイトルが設定されていること（空でないこと）
  const title = await page.title();
  expect(title.length).toBeGreaterThan(0); // 【確認内容】: タイトルが空でない
});

// -------------------------------------------------------------------------
// TC-1502-E02: セキュリティヘッダー確認
// -------------------------------------------------------------------------

test('TC-1502-E03: Cross-Origin-Opener-Policy: same-origin ヘッダーが設定されている', async ({ page }) => {
  // 【テスト目的】: SharedArrayBuffer 使用に必要な COOP ヘッダーを確認 🟢 REQ-001
  // 【テスト内容】: HTTP レスポンスヘッダーに COOP が含まれることを確認
  // 【期待される動作】: Cross-Origin-Opener-Policy: same-origin が設定されている

  const response = await page.goto('/');

  // 【結果検証】: COOP ヘッダーが正しく設定されていること
  const coop = response?.headers()['cross-origin-opener-policy'];
  expect(coop).toBe('same-origin'); // 【確認内容】: SharedArrayBuffer 有効化に必要なヘッダー
});

test('TC-1502-E04: Cross-Origin-Embedder-Policy: require-corp ヘッダーが設定されている', async ({ page }) => {
  // 【テスト目的】: SharedArrayBuffer 使用に必要な COEP ヘッダーを確認 🟢 REQ-002
  // 【テスト内容】: HTTP レスポンスヘッダーに COEP が含まれることを確認
  // 【期待される動作】: Cross-Origin-Embedder-Policy: require-corp が設定されている

  const response = await page.goto('/');

  // 【結果検証】: COEP ヘッダーが正しく設定されていること
  const coep = response?.headers()['cross-origin-embedder-policy'];
  expect(coep).toBe('require-corp'); // 【確認内容】: SharedArrayBuffer 有効化に必要なヘッダー
});

// -------------------------------------------------------------------------
// TC-1502-E03: SharedArrayBuffer 利用可能確認
// -------------------------------------------------------------------------

test('TC-1502-E05: SharedArrayBuffer がブラウザで利用可能である', async ({ page }) => {
  // 【テスト目的】: COOP/COEP ヘッダーにより SharedArrayBuffer が有効化されていることを確認 🟢
  // 【テスト内容】: ブラウザ内で SharedArrayBuffer が typeof undefined でないことを確認
  // 【期待される動作】: SharedArrayBuffer が利用可能

  await page.goto('/');

  // 【結果検証】: SharedArrayBuffer が定義されていること
  const isAvailable = await page.evaluate(() => typeof SharedArrayBuffer !== 'undefined');
  expect(isAvailable).toBe(true); // 【確認内容】: WASM SharedMemory に必要
});

// -------------------------------------------------------------------------
// TC-1502-E04: アプリ基本構造確認
// -------------------------------------------------------------------------

test('TC-1502-E06: HTML に script タグが含まれている（アプリが初期化される）', async ({ page }) => {
  // 【テスト目的】: HTML にアプリスクリプトが含まれていることを確認 🟢
  // 【テスト内容】: ページ内に少なくとも 1 つの script タグが存在すること
  // 【期待される動作】: Vite でビルドされたスクリプトが読み込まれる

  await page.goto('/');

  // 【結果検証】: script タグが存在すること
  const scriptCount = await page.locator('script').count();
  expect(scriptCount).toBeGreaterThan(0); // 【確認内容】: アプリスクリプトが読み込まれている
});
