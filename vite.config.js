import { resolve } from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  root: 'src',
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: 'es2022',
    rollupOptions: {
      input: {
        overlay: resolve('src/overlay.html'),
        toast: resolve('src/toast.html'),
        editor: resolve('src/editor.html'),
        hub: resolve('src/hub.html'),
        shelf: resolve('src/shelf.html'),
        peek: resolve('src/peek.html'),
      },
    },
  },
});
