import { defineConfig } from 'eslint/config';
import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import globals from 'globals';
import eslintConfigPrettier from 'eslint-config-prettier/flat';
import eslintPluginReact from 'eslint-plugin-react';
import eslintPluginReactHooks from 'eslint-plugin-react-hooks';
import eslintPluginReactRefresh from 'eslint-plugin-react-refresh';

export default defineConfig(
  {
    ignores: [
      '**/node_modules',
      '**/dist',
      '**/out',
      '**/release',
      'src-tauri/target',
      'src-tauri/gen',
      // 외부에서 가져온 스킬은 원본 그대로 둔다
      '.agents',
      // ts-rs가 Rust 타입에서 만든 파일
      'src/renderer/src/types/bindings',
      // spec-kit autopilot 루프 기록. 테스트 입력 묶음과 snapshot에 소스 사본이 들어 있다
      'specs/*/loop',
    ],
  },
  js.configs.recommended,
  tseslint.configs.recommended,

  // 설정 파일: Node 환경
  {
    files: ['*.{js,mjs,cjs,ts}'],
    languageOptions: {
      globals: globals.node,
    },
  },

  // renderer: 브라우저 + React
  {
    files: ['src/renderer/**/*.{ts,tsx}'],
    ...eslintPluginReact.configs.flat.recommended,
    ...eslintPluginReact.configs.flat['jsx-runtime'],
    languageOptions: {
      ...eslintPluginReact.configs.flat.recommended.languageOptions,
      globals: globals.browser,
    },
    settings: {
      react: { version: 'detect' },
    },
    plugins: {
      react: eslintPluginReact,
      'react-hooks': eslintPluginReactHooks,
      'react-refresh': eslintPluginReactRefresh,
    },
    rules: {
      ...eslintPluginReact.configs.flat.recommended.rules,
      ...eslintPluginReact.configs.flat['jsx-runtime'].rules,
      ...eslintPluginReactHooks.configs.recommended.rules,
      ...eslintPluginReactRefresh.configs.vite.rules,
      // R3F의 three 속성(args, intensity 등)을 DOM 속성으로 오인한다. JSX 속성은 TypeScript가 검사한다
      'react/no-unknown-property': 'off',
    },
  },

  // macOS 빌드 타깃(safari13)에 없는 API. esbuild가 polyfill하지 않아 런타임에 깨진다
  {
    files: ['src/renderer/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-properties': [
        'error',
        ...[
          'toSorted',
          'toReversed',
          'toSpliced',
          'with',
          'findLast',
          'findLastIndex',
          'at',
          'replaceAll',
        ].map((property) => ({
          property,
          message: 'safari13에 없는 메서드입니다. AGENTS.md "구조"의 웹뷰 항목을 보세요.',
        })),
        { object: 'Object', property: 'hasOwn', message: 'safari13에 없습니다.' },
        { object: 'Promise', property: 'any', message: 'safari13에 없습니다.' },
        { object: 'crypto', property: 'randomUUID', message: 'safari13에 없습니다.' },
      ],
      'no-restricted-globals': [
        'error',
        { name: 'structuredClone', message: 'safari13에 없습니다.' },
        { name: 'requestIdleCallback', message: 'safari13에 없습니다.' },
      ],
    },
  },

  // domain/(core)은 UI, 상태 저장소, 렌더링, 플랫폼, IO 경계를 import하지 않는다
  {
    files: ['src/renderer/**/domain/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: [
                'react',
                'react-dom',
                'react/*',
                'react-dom/*',
                'zustand',
                'zustand/*',
                'three',
                'three/*',
              ],
              message: 'domain/은 순수 TS만 둡니다. AGENTS.md "폴더 구조"를 보세요.',
            },
            {
              group: ['@react-three/*', '@tauri-apps/*', 'leva'],
              message: 'domain/은 순수 TS만 둡니다.',
            },
            {
              group: ['@renderer/api', '@renderer/api/*', '**/api', '**/api/*'],
              message: 'domain/은 IO 경계를 부르지 않습니다.',
            },
          ],
        },
      ],
    },
  },

  {
    rules: {
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },

  // Prettier와 충돌하는 포맷 관련 규칙 비활성화 (항상 마지막에 위치)
  eslintConfigPrettier,
);
