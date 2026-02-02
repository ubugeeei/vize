import { defineConfig } from 'vite';
import { vize } from '@vizejs/vite-plugin';
import { resolve } from 'path';

export default defineConfig({
  plugins: [
    vize(),
  ],
  resolve: {
    alias: {
      // Map vue to full build (includes compiler)
      vue: 'vue/dist/vue.esm-bundler.js',
      // Shim for SSR imports (script setup generates these)
      '@vue/runtime-core/server-renderer': resolve(__dirname, 'ssr-shim.ts'),
    },
  },
  build: {
    target: 'node18',
    lib: {
      entry: 'main.ts',
      formats: ['es'],
      fileName: 'main',
    },
    rollupOptions: {
      external: ['@vizejs/fresco-native', '@vizejs/fresco'],
    },
  },
});
