import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// ARCHITECTURE.md §4: app/ is the Svelte 5 frontend workspace.
// Tauri integration (src-tauri, IPC) lands with task P0.4.
export default defineConfig({
  plugins: [svelte()],
  // $lib alias (SvelteKit convention) for vendored component imports.
  resolve: {
    alias: {
      $lib: new URL('./src/lib', import.meta.url).pathname,
    },
  },
  // Strict TS lives in tsconfig.json; Vite serves app root only.
  server: {
    strictPort: true,
    port: 5173,
  },
  build: {
    target: 'es2022',
  },
});
