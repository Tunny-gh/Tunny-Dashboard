/**
 * ConvergenceDiagnosis テスト (TASK-1001)
 *
 * 【テスト対象】: ConvergenceDiagnosis — 収束診断パネル
 * 【テスト方針】: diagnoseConvergence() の判定ロジック（境界値）と
 *               バッジ表示（converged/converging/not-converged/insufficient）を検証する
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup } from '@testing-library/react';

import { ConvergenceDiagnosis, diagnoseConvergence } from './ConvergenceDiagnosis';
import type { TrialData } from '../charts/OptimizationHistory';

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/** 【ヘルパー】: 収束済みに見えるデータを生成する（最後の20%で改善なし） */
function makeConvergedData(total: number): TrialData[] {
  return Array.from({ length: total }, (_, i) => ({
    trial: i + 1,
    // 最初の80%は急速に改善、最後の20%はほぼ変化なし
    value: i < total * 0.8 ? 100 - i * 1.0 : 100 - total * 0.8,
  }));
}

/** 【ヘルパー】: 改善中のデータを生成する（全体で緩やかに改善） */
function makeConvergingData(total: number): TrialData[] {
  return Array.from({ length: total }, (_, i) => ({
    trial: i + 1,
    value: 100 - i * 0.3,
  }));
}

/** 【ヘルパー】: 試行数が少ないデータ（判定不可）を生成する */
function makeInsufficientData(count: number): TrialData[] {
  return Array.from({ length: count }, (_, i) => ({
    trial: i + 1,
    value: 100 - i,
  }));
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ConvergenceDiagnosis — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // TC-1001-07: 試行数不足で「判定不可」を表示
  test('TC-1001-07: data=[](試行数不足) のとき「判定不可（試行数不足）」を表示する', () => {
    // 【テスト目的】: 試行数が少ない場合に適切なメッセージが表示されること 🟢
    render(<ConvergenceDiagnosis data={[]} direction="minimize" />);

    // 【確認内容】: 「判定不可（試行数不足）」メッセージが表示されること
    expect(screen.getByText('判定不可（試行数不足）')).toBeInTheDocument();
  });

  // TC-1001-08: 収束済みデータで converged バッジを表示
  test('TC-1001-08: 収束済みデータで data-testid=badge-converged が表示される', () => {
    // 【テスト目的】: 収束済み判定時に緑バッジが表示されること 🟢
    render(<ConvergenceDiagnosis data={makeConvergedData(30)} direction="minimize" />);

    // 【確認内容】: 収束済みバッジが表示されること
    expect(screen.getByTestId('badge-converged')).toBeInTheDocument();
  });

  // TC-1001-09: 収束中データで converging バッジを表示
  test('TC-1001-09: 収束中データで data-testid=badge-converging が表示される', () => {
    // 【テスト目的】: 収束中判定時に黄バッジが表示されること 🟢
    render(<ConvergenceDiagnosis data={makeConvergingData(20)} direction="minimize" />);

    // 【確認内容】: 収束中バッジが表示されること
    expect(screen.getByTestId('badge-converging')).toBeInTheDocument();
  });
});

// -------------------------------------------------------------------------
// diagnoseConvergence — 収束判定ロジック
// -------------------------------------------------------------------------

describe('diagnoseConvergence — 収束判定ロジック', () => {
  // TC-1001-10: 試行数 < 10 で 'insufficient' を返す
  test('TC-1001-10: data.length=5 のとき insufficient が返される', () => {
    // 【テスト目的】: 試行数が 10 未満の場合は判定不可を返すこと 🟢
    const result = diagnoseConvergence(makeInsufficientData(5), 'minimize');
    expect(result).toBe('insufficient'); // 【確認内容】: 試行数不足でinsufficientが返ること
  });

  // TC-1001-11: 収束済みデータで 'converged' を返す
  test('TC-1001-11: 最後の20%で改善なしのデータで converged が返される', () => {
    // 【テスト目的】: 末尾 20% で改善がない場合は収束済みを返すこと 🟢
    const result = diagnoseConvergence(makeConvergedData(30), 'minimize');
    expect(result).toBe('converged'); // 【確認内容】: 収束済みデータでconvergedが返ること
  });

  // TC-1001-12: 改善中データで 'converging' または 'not-converged' を返す
  test('TC-1001-12: 緩やかに改善中のデータで converged にはならない', () => {
    // 【テスト目的】: 継続的に改善しているデータは収束済みにならないこと 🟢
    const result = diagnoseConvergence(makeConvergingData(20), 'minimize');
    expect(result).not.toBe('converged'); // 【確認内容】: 改善継続中はconvergedにならないこと
    expect(result).not.toBe('insufficient'); // 【確認内容】: 試行数は十分なのでinsufficientにならないこと
  });
});
