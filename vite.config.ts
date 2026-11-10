import { resolve } from 'node:path';
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// Tauri CLI가 beforeDevCommand / beforeBuildCommand를 실행할 때 넣어 주는 환경 변수
const platform = process.env['TAURI_ENV_PLATFORM'];
const isDebug = !!process.env['TAURI_ENV_DEBUG'];

// Renderer(React): 일반 웹앱과 동일하게 빌드하고, Tauri가 out/renderer를 앱에 넣는다.
export default defineConfig(({ command }) => ({
  root: resolve(__dirname, 'src/renderer'),
  base: './',
  plugins: [react()],
  // Rust 컴파일 로그가 지워지지 않게 한다.
  clearScreen: false,
  resolve: {
    alias: {
      '@renderer': resolve(__dirname, 'src/renderer/src'),
      // leva(개발용 조절 패널)는 빌드에서 스텁으로 바꿔 번들에서 뺀다.
      // leva에 sideEffects 표시가 없어 import.meta.env.DEV 조건만으로는 빠지지 않는다.
      ...(command === 'build' && {
        leva: resolve(__dirname, 'src/renderer/src/dev/leva-stub.ts'),
      }),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    outDir: resolve(__dirname, 'out/renderer'),
    emptyOutDir: true,
    // 웹뷰가 OS마다 다르다: Windows는 WebView2(Chromium), macOS는 WKWebView(Safari)
    target: platform === 'windows' ? 'chrome105' : 'safari13',
    minify: isDebug ? false : 'esbuild',
    sourcemap: isDebug,
  },
  // Vitest: 경로는 root(src/renderer) 기준. 테스트 파일은 소스 옆에 *.test.ts(x)로 둔다
  test: {
    include: ['src/**/*.test.{ts,tsx}'],
    environment: 'node',
    // 아직 테스트가 없어도 CI가 실패하지 않게 한다
    passWithNoTests: true,
  },
}));
