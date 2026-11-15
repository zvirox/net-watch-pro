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
      // Typed linting for .ts files; config .js files sit outside the
      // tsconfig project, so they get the default project instead.
      parserOptions: {
        projectService: {
          allowDefaultProject: ['eslint.config.js', 'svelte.config.js'],
        },
        extraFileExtensions: ['.svelte'],
      },
    },
    rules: {
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      '@typescript-eslint/no-explicit-any': 'error',
    },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      // eslint-plugin-svelte parses the file; TS script contents need the
      // TS parser explicitly (documented setup for Svelte 5 + TS).
      parserOptions: {
        parser: ts.parser,
        projectService: true,
        extraFileExtensions: ['.svelte'],
      },
    },
  },
  {
    ignores: ['dist/', 'node_modules/', 'src-tauri/'],
  },
);
