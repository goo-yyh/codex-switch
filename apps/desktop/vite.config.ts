import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
export default defineConfig({
  plugins: [react()],
  server: { port: 1420, strictPort: true },
  envPrefix: 'VITE_PUBLIC_',
  test: { environment: 'jsdom', setupFiles: ['./src/test-setup.ts'] },
});
