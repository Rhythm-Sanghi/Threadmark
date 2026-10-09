import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  // Keep unrelated PostCSS configuration in parent directories out of this build.
  css: { postcss: { plugins: [] } },
  server: { host: '127.0.0.1', port: 1420, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } },
  build: { target: 'es2022' },
  test: { environment: 'jsdom', restoreMocks: true },
});
