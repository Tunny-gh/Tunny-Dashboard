/**
 * ArtifactStore テスト (TASK-1301)
 *
 * 【テスト対象】: ArtifactStore + ユーティリティ
 * 【テスト方針】: showDirectoryPicker / FileSystemDirectoryHandle を vi.fn でスタブ
 */

import { describe, test, expect, beforeEach, vi } from 'vitest';
import { useArtifactStore, getMimeTypeCategory, buildArtifactMeta } from './artifactStore';

// -------------------------------------------------------------------------
// DOM スタブ
// -------------------------------------------------------------------------

Object.defineProperty(globalThis, 'URL', {
  value: {
    createObjectURL: vi.fn(() => 'blob:mock-url'),
    revokeObjectURL: vi.fn(),
  },
  writable: true,
});

// -------------------------------------------------------------------------
// ヘルパー
// -------------------------------------------------------------------------

/** モックの FileSystemFileHandle を生成する */
function makeFileHandle() {
  return {
    getFile: vi.fn().mockResolvedValue(new Blob(['test content'])),
  } as unknown as FileSystemFileHandle;
}

/** モックの FileSystemDirectoryHandle を生成する */
function makeDirHandle(files: Record<string, FileSystemFileHandle> = {}) {
  return {
    kind: 'directory',
    name: 'artifacts',
    getFileHandle: vi.fn().mockImplementation((name: string, _opts?: object) => {
      const handle = files[name];
      if (handle) return Promise.resolve(handle);
      return Promise.reject(new DOMException('File not found', 'NotFoundError'));
    }),
  } as unknown as FileSystemDirectoryHandle;
}

/** Store を初期状態にリセットする */
function resetStore() {
  useArtifactStore.setState({
    dirHandle: null,
    urlCache: new Map(),
    isPickingDir: false,
    error: null,
  });
}

// -------------------------------------------------------------------------
// ユーティリティ関数テスト
// -------------------------------------------------------------------------

describe('getMimeTypeCategory', () => {
  // TC-1301-U01: 画像拡張子の判定
  test('TC-1301-U01: .png は image を返す', () => {
    // 【テスト目的】: 画像ファイルが正しく分類されることを確認 🟢
    expect(getMimeTypeCategory('photo.png')).toBe('image');
    expect(getMimeTypeCategory('photo.jpg')).toBe('image');
    expect(getMimeTypeCategory('photo.gif')).toBe('image');
  });

  // TC-1301-U02: CSV 拡張子の判定
  test('TC-1301-U02: .csv は csv を返す', () => {
    // 【テスト目的】: CSV ファイルが正しく分類されることを確認 🟢
    expect(getMimeTypeCategory('data.csv')).toBe('csv');
  });

  // TC-1301-U03: JSON 拡張子の判定
  test('TC-1301-U03: .json は json を返す', () => {
    // 【テスト目的】: JSON ファイルが正しく分類されることを確認 🟢
    expect(getMimeTypeCategory('result.json')).toBe('json');
  });

  // TC-1301-U04: 未知の拡張子は other
  test('TC-1301-U04: 未知の拡張子は other を返す', () => {
    // 【テスト目的】: 未知の拡張子が other に分類されることを確認 🟢
    expect(getMimeTypeCategory('file.xyz')).toBe('other');
    expect(getMimeTypeCategory('noextension')).toBe('other');
  });
});

describe('buildArtifactMeta', () => {
  // TC-1301-U05: ArtifactMeta が正しく構築される
  test('TC-1301-U05: buildArtifactMeta が正しい meta を返す', () => {
    // 【テスト目的】: メタデータ構築が正しく動作することを確認 🟢
    const meta = buildArtifactMeta('abc123', 'chart.png', 42);
    expect(meta.artifactId).toBe('abc123');
    expect(meta.filename).toBe('chart.png');
    expect(meta.trialId).toBe(42);
    expect(meta.mimetype).toBe('image/*');
  });
});

// -------------------------------------------------------------------------
// ArtifactStore テスト
// -------------------------------------------------------------------------

describe('ArtifactStore — ディレクトリ選択', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
  });

  // TC-1301-D01: pickDirectory 成功後 dirHandle が設定される
  test('TC-1301-D01: pickDirectory 成功後 dirHandle が設定される', async () => {
    // 【テスト目的】: ディレクトリ選択後に dirHandle がセットされることを確認 🟢
    const mockHandle = makeDirHandle();
    (window as Record<string, unknown>).showDirectoryPicker = vi.fn().mockResolvedValue(mockHandle);

    const result = await useArtifactStore.getState().pickDirectory();

    // 【確認内容】: true が返ること
    expect(result).toBe(true);
    // 【確認内容】: dirHandle が設定されること
    expect(useArtifactStore.getState().dirHandle).toBe(mockHandle);
  });

  // TC-1301-D02: pickDirectory キャンセル時は false が返る
  test('TC-1301-D02: AbortError キャンセルで false が返る', async () => {
    // 【テスト目的】: ユーザーキャンセル時にエラーなく false が返ることを確認 🟢
    const abort = new DOMException('User cancelled', 'AbortError');
    (window as Record<string, unknown>).showDirectoryPicker = vi
      .fn()
      .mockRejectedValue(abort);

    const result = await useArtifactStore.getState().pickDirectory();

    // 【確認内容】: false が返ること
    expect(result).toBe(false);
    // 【確認内容】: error が設定されないこと
    expect(useArtifactStore.getState().error).toBeNull();
  });

  // TC-1301-D03: 非対応環境で error がセットされる
  test('TC-1301-D03: showDirectoryPicker がない環境で error がセットされる', async () => {
    // 【テスト目的】: FSAPI 非対応環境でエラーが設定されることを確認 🟢
    const orig = (window as Record<string, unknown>).showDirectoryPicker;
    delete (window as Record<string, unknown>).showDirectoryPicker;

    const result = await useArtifactStore.getState().pickDirectory();

    // 【確認内容】: false が返ること
    expect(result).toBe(false);
    // 【確認内容】: error が設定されること
    expect(useArtifactStore.getState().error).toContain('対応していません');

    if (orig) (window as Record<string, unknown>).showDirectoryPicker = orig;
  });
});

describe('ArtifactStore — アーティファクト URL ロード', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
  });

  // TC-1301-L01: ファイルあり時に ObjectURL が返る
  test('TC-1301-L01: ファイルが存在する場合 ObjectURL が返る', async () => {
    // 【テスト目的】: ファイルが見つかった場合に ObjectURL が返ることを確認 🟢
    const fileHandle = makeFileHandle();
    const dirHandle = makeDirHandle({ 'abc.png': fileHandle });
    useArtifactStore.setState({ dirHandle });

    const url = await useArtifactStore.getState().loadArtifactUrl('abc123', 'abc.png');

    // 【確認内容】: URL が返ること
    expect(url).toBe('blob:mock-url');
    // 【確認内容】: キャッシュに保存されること
    expect(useArtifactStore.getState().urlCache.get('abc123')).toBe('blob:mock-url');
  });

  // TC-1301-L02: ファイルなし時に null が返る
  test('TC-1301-L02: ファイルが存在しない場合 null が返る', async () => {
    // 【テスト目的】: ファイルが見つからない場合に null が返ることを確認 🟢 REQ-141
    const dirHandle = makeDirHandle({}); // ファイルなし
    useArtifactStore.setState({ dirHandle });

    const url = await useArtifactStore.getState().loadArtifactUrl('missing', 'missing.png');

    // 【確認内容】: null が返ること
    expect(url).toBeNull();
  });

  // TC-1301-L03: キャッシュ済みは2回目の取得で再利用される
  test('TC-1301-L03: キャッシュ済みの URL は再ロードせず返される', async () => {
    // 【テスト目的】: キャッシュが有効に機能することを確認 🟢
    const initialCache = new Map([['cached123', 'blob:cached-url']]);
    useArtifactStore.setState({
      dirHandle: makeDirHandle({}),
      urlCache: initialCache,
    });

    const url = await useArtifactStore.getState().loadArtifactUrl('cached123', 'any.png');

    // 【確認内容】: キャッシュの URL が返ること
    expect(url).toBe('blob:cached-url');
  });

  // TC-1301-L04: dirHandle なし時に null が返る
  test('TC-1301-L04: dirHandle が null のとき loadArtifactUrl は null を返す', async () => {
    // 【テスト目的】: ディレクトリ未選択時に null が返ることを確認 🟢
    useArtifactStore.setState({ dirHandle: null });
    const url = await useArtifactStore.getState().loadArtifactUrl('any', 'any.png');
    expect(url).toBeNull();
  });
});

describe('ArtifactStore — releaseAll', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetStore();
  });

  // TC-1301-R01: releaseAll で urlCache がクリアされ revokeObjectURL が呼ばれる
  test('TC-1301-R01: releaseAll で全 URL が解放される', () => {
    // 【テスト目的】: releaseAll でメモリが正しく解放されることを確認 🟢
    const cache = new Map([['a', 'blob:url1'], ['b', 'blob:url2']]);
    useArtifactStore.setState({ urlCache: cache });

    useArtifactStore.getState().releaseAll();

    // 【確認内容】: urlCache が空になること
    expect(useArtifactStore.getState().urlCache.size).toBe(0);
    // 【確認内容】: revokeObjectURL が 2 回呼ばれること
    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(2);
  });
});
