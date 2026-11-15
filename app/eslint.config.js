// ESLint flat config — ARCHITECTURE.md §8.2 (strict TS, no any).
// globals via browser assumptions; svelte plugin for .svelte files.
import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';

export default ts.config(
  js.configs.recommended,
  ...ts.configs.strictTypeChecked,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: { window: 'readonly', document: 'readonly', console: 'readonly' },
      // Typed linting for .ts and .svelte alike (tseslint projectService).
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
      },
    },
    rules: {
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      '@typescript-eslint/no-explicit-any': 'error',
    },
  },
  {
    ignores: ['dist/', 'node_modules/', 'src-tauri/'],
  },
);
