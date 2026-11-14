import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { requiredAppIcon } from './build/requiredAppIcon.mjs';

export default defineConfig({
  plugins: [react(), requiredAppIcon()],
  clearScreen: false,
  server: { host: '127.0.0.1', port: 8922, strictPort: true },
  build: { target: 'es2022' },
});
