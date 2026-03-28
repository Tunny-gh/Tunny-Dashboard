/**
 * ArtifactGallery テスト (TASK-1301)
 *
 * 【テスト対象】: ArtifactGallery コンポーネント
 * 【テスト方針】: artifactStore を vi.mock でスタブ
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import '@testing-library/jest-dom';

// -------------------------------------------------------------------------
// artifactStore モック
// -------------------------------------------------------------------------

vi.mock('../../stores/artifactStore', () => ({
  useArtifactStore: vi.fn(() => ({
    dirHandle: { name: 'artifacts' } as unknown as FileSystemDirectoryHandle,
    loadArtifactUrl: vi.fn().mockResolvedValue(null),
    urlCache: new Map(),
  })),
  getMimeTypeCategory: (filename: string) => {
    const ext = filename.split('.').pop()?.toLowerCase() ?? '';
    if (['png', 'jpg'].includes(ext)) return 'image';
    return 'other';
  },
}));

import { ArtifactGallery } from './ArtifactGallery';
import type { Trial } from '../../types';
import { useArtifactStore } from '../../stores/artifactStore';

// -------------------------------------------------------------------------
// ヘルパー
// -------------------------------------------------------------------------

function makeTrial(trialId: number, hasArtifact = true): Trial {
  return {
    trialId,
    state: 'COMPLETE',
    params: {},
    values: [1.0],
    clusterId: null,
    isFeasible: true,
    userAttrs: {},
    artifactIds: hasArtifact ? [`artifact-${trialId}.png`] : [],
  };
}

// -------------------------------------------------------------------------
// テスト
// -------------------------------------------------------------------------

describe('ArtifactGallery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // モックをデフォルト状態にリセット（TC-1301-G01 が dirHandle=null に変更するため）
    (useArtifactStore as ReturnType<typeof vi.fn>).mockReturnValue({
      dirHandle: { name: 'artifacts' } as unknown as FileSystemDirectoryHandle,
      loadArtifactUrl: vi.fn().mockResolvedValue(null),
      urlCache: new Map(),
    });
  });

  // TC-1301-G01: ディレクトリ未選択時は表示されない
  test('TC-1301-G01: dirHandle=null のとき ArtifactGallery は非表示', () => {
    // 【テスト目的】: ディレクトリ未選択時にギャラリーが非表示になることを確認 🟢 REQ-140
    (useArtifactStore as ReturnType<typeof vi.fn>).mockReturnValue({
      dirHandle: null,
      loadArtifactUrl: vi.fn(),
    });
    render(<ArtifactGallery trials={[makeTrial(1)]} />);
    expect(screen.queryByTestId('artifact-gallery')).not.toBeInTheDocument();
  });

  // TC-1301-G02: アーティファクト付き trial がカードで表示される
  test('TC-1301-G02: アーティファクトある trial のカードが表示される', () => {
    // 【テスト目的】: アーティファクト付き試行がカードとして表示されることを確認 🟢
    const trials = [makeTrial(1), makeTrial(2), makeTrial(3)];
    render(<ArtifactGallery trials={trials} />);
    // 【確認内容】: 3件のカードが表示されること
    expect(screen.getByTestId('gallery-card-1')).toBeInTheDocument();
    expect(screen.getByTestId('gallery-card-2')).toBeInTheDocument();
    expect(screen.getByTestId('gallery-card-3')).toBeInTheDocument();
  });

  // TC-1301-G03: アーティファクトなし trial は表示されない
  test('TC-1301-G03: アーティファクトなし trial は gallery に表示されない', () => {
    // 【テスト目的】: アーティファクトなし試行がフィルタされることを確認 🟢
    const trials = [makeTrial(1, true), makeTrial(2, false)];
    render(<ArtifactGallery trials={trials} />);
    // 【確認内容】: trial 1 は表示されること
    expect(screen.getByTestId('gallery-card-1')).toBeInTheDocument();
    // 【確認内容】: trial 2 は表示されないこと（アーティファクトなし）
    expect(screen.queryByTestId('gallery-card-2')).not.toBeInTheDocument();
  });

  // TC-1301-G04: カードサイズ切替ボタンが表示される
  test('TC-1301-G04: カードサイズ切替ボタン（小/中/大）が表示される', () => {
    // 【テスト目的】: サイズ切替 UI が表示されることを確認 🟢
    render(<ArtifactGallery trials={[makeTrial(1)]} />);
    expect(screen.getByTestId('card-size-small')).toBeInTheDocument();
    expect(screen.getByTestId('card-size-medium')).toBeInTheDocument();
    expect(screen.getByTestId('card-size-large')).toBeInTheDocument();
  });

  // TC-1301-G05: 49件以上で「さらに読み込む」ボタンが表示される
  test('TC-1301-G05: 49 件のとき「さらに読み込む」ボタンが表示される', () => {
    // 【テスト目的】: 48 件超でページングボタンが表示されることを確認 🟡
    const trials = Array.from({ length: 49 }, (_, i) => makeTrial(i + 1));
    render(<ArtifactGallery trials={trials} />);
    // 【確認内容】: 「さらに読み込む」ボタンが表示されること
    expect(screen.getByTestId('load-more-btn')).toBeInTheDocument();
  });

  // TC-1301-G06: 「さらに読み込む」ボタンクリックでさらに表示される
  test('TC-1301-G06: 「さらに読み込む」クリックで表示件数が増える', () => {
    // 【テスト目的】: ページング操作が正しく機能することを確認 🟡
    const trials = Array.from({ length: 49 }, (_, i) => makeTrial(i + 1));
    render(<ArtifactGallery trials={trials} />);

    // ボタンクリック前: 48 件
    expect(screen.getByTestId('gallery-card-48')).toBeInTheDocument();
    expect(screen.queryByTestId('gallery-card-49')).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId('load-more-btn'));

    // ボタンクリック後: 49 件すべて表示
    expect(screen.getByTestId('gallery-card-49')).toBeInTheDocument();
  });

  // TC-1301-G07: 0件のとき空メッセージが表示される
  test('TC-1301-G07: アーティファクトなし trial のみのとき gallery-empty が表示される', () => {
    // 【テスト目的】: 表示件数0のとき空メッセージが表示されることを確認 🟢
    render(<ArtifactGallery trials={[makeTrial(1, false)]} />);
    expect(screen.getByTestId('gallery-empty')).toBeInTheDocument();
  });
});
