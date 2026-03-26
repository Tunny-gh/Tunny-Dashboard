import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { viteSingleFile } from 'vite-plugin-singlefile'
import path from 'path'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), viteSingleFile()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    headers: {
      // SharedArrayBuffer を使用するために必要なセキュリティヘッダー
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  preview: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  build: {
    target: 'esnext', // WASM + top-level await に必要
    // viteSingleFile がインライン化するため assetsInlineLimit を大きく設定
    // WASMも base64 としてHTMLに埋め込む（最終ビルドサイズは増えるが配布が単一ファイルになる）
    assetsInlineLimit: 100 * 1024 * 1024, // 100MB: 実質すべてのアセットをインライン化
    rollupOptions: {
      output: {
        manualChunks: undefined,
      },
    },
  },
  // WASMファイルをアセットとして扱う
  assetsInclude: ['**/*.wasm'],
  optimizeDeps: {
    exclude: ['tunny-core'], // WASMクレートはbundlerに含めない
  },
})
