import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath } from 'node:url';

// ARCHITECTURE.md §4: app/ is the React 19 + Tailwind v4 frontend workspace.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '$lib': fileURLToPath(new URL('./src/lib', import.meta.url)),
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
