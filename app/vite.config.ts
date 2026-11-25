import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

const tauriConfPath = fileURLToPath(new URL('./src-tauri/tauri.conf.json', import.meta.url));
const tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf8')) as { version?: string };

// ARCHITECTURE.md §4: app/ is the React 19 + Tailwind v4 frontend workspace.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {
    __NWP_VERSION__: JSON.stringify(tauriConf.version ?? '0.9.2-beta.5'),
  },
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    strictPort: true,
    port: 5173,
  },
  build: {
    target: 'es2022',
  },
});
