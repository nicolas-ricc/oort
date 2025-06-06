import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import tailwindcss from "@tailwindcss/vite"
import postcss from "@tailwindcss/postcss"
// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  css: {
    postcss: {
      plugins: [postcss]
    }
  },
  server: {
    host: '0.0.0.0',
    port: 5173
  },
  rollupOptions: {
    external: ["react", /^react\/.*/, "react-dom", /react-dom\/.*/],
    output: {
      globals: {
        'react-dom': 'ReactDom',
        react: 'React',
        'react/jsx-runtime': 'ReactJsxRuntime',
      },
    },
  }
})
