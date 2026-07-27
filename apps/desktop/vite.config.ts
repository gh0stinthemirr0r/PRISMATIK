import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true
  },
  envPrefix: ['VITE_', 'TAURI_'],
  optimizeDeps: {
    include: ['chroma-js'],
    needsInterop: ['chroma-js'],
    exclude: [
      '@finos/perspective'
    ]
  },
  worker: {
    format: 'es'
  },
  build: {
    // Perspective WASM + workers need a modern target; Tauri Windows WebView2 is Chromium-based.
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'esnext',
    minify: process.env.TAURI_ENV_DEBUG ? false : 'esbuild',
    sourcemap: !!process.env.TAURI_ENV_DEBUG
  },
  assetsInclude: ['**/*.wasm']
});
