import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import { resolve } from 'path'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  
  return {
    plugins: [react()],
    
    define: {
      __APP_VERSION__: JSON.stringify(process.env.npm_package_version),
      __BUILD_TIME__: JSON.stringify(new Date().toISOString()),
      __MODE__: JSON.stringify(mode)
    },
    
    server: {
      port: parseInt(env.FRONTEND_PORT) || 5173,
      host: '0.0.0.0',
      strictPort: true,
      proxy: {
        '/api': {
          target: env.BACKEND_URL || 'http://localhost:3001',
          changeOrigin: true,
          secure: false
        },
        '/ws': {
          target: env.BACKEND_WS_URL || 'ws://localhost:3001',
          ws: true,
          changeOrigin: true
        },
        '/pty': {
          target: env.BACKEND_WS_URL || 'ws://localhost:3001',
          ws: true,
          changeOrigin: true
        },
        '/health': {
          target: env.BACKEND_URL || 'http://localhost:3001',
          changeOrigin: true,
          secure: false
        }
      },
      cors: true
    },
    
    preview: {
      port: parseInt(env.PREVIEW_PORT) || 4173,
      host: '0.0.0.0',
      strictPort: true
    },
    
    build: {
      outDir: 'dist',
      sourcemap: mode === 'development',
      minify: mode === 'production' ? 'esbuild' : false,
      target: 'es2020',
      chunkSizeWarningLimit: 2000,
      rollupOptions: {
        input: {
          main: resolve(__dirname, 'index.html')
        },
        output: {
          manualChunks: {
            vendor: ['react', 'react-dom'],
            xterm: ['xterm', 'xterm-addon-fit', 'xterm-addon-web-links', 'xterm-addon-attach'],
            utils: ['axios']
          },
          chunkFileNames: 'assets/[name]-[hash].js',
          entryFileNames: 'assets/[name]-[hash].js',
          assetFileNames: 'assets/[name]-[hash].[ext]'
        }
      },
      reportCompressedSize: true,
      emptyOutDir: true
    },
    
    resolve: {
      alias: {
        '@': resolve(__dirname, 'src'),
        '@components': resolve(__dirname, 'src/components'),
        '@services': resolve(__dirname, 'src/services'),
        '@types': resolve(__dirname, 'src/types')
      }
    },
    
    optimizeDeps: {
      include: [
        'react',
        'react-dom',
        'xterm',
        'xterm-addon-fit',
        'xterm-addon-web-links',
        'xterm-addon-attach',
        'axios'
      ],
      force: mode === 'development'
    },
    
    css: {
      devSourcemap: mode === 'development',
      postcss: './postcss.config.js'
    },
    
    esbuild: {
      drop: mode === 'production' ? ['console', 'debugger'] : [],
      target: 'es2020'
    }
  }
})