import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
    fs: {
      allow: [fileURLToPath(new URL('../..', import.meta.url))]
    },
    watch: {
      // `fs.allow` opens the whole repository so workspace packages resolve,
      // and the watcher follows it. That is fatal on a default Linux inotify
      // budget: `references/` alone is ~1.8GB across roughly 23,600 files, so
      // the walk exhausts all 65,536 watches and `tauri dev` dies with
      // "OS file watch limit reached" — an error that points at Cargo.toml
      // and says nothing about the real cause.
      //
      // None of these paths can trigger a frontend rebuild, so ignoring them
      // costs nothing and lets the project start on an untouched machine
      // rather than requiring every developer to raise a sysctl first.
      ignored: [
        '**/references/**',
        '**/DOCS/**',
        '**/DOCS2/**',
        '**/legacy-v0.1/**',
        '**/services/**',
        '**/target/**',
        '**/.git/**',
        '**/artifacts/**',
        '**/supply-chain/**',
      ],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  optimizeDeps: {
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
