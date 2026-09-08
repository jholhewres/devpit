import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

/**
 * 17800 rather than Vite's 5173.
 *
 * The default port is contested on any machine running more than one project,
 * and a collision here is not a loud failure — it is a window pointed at
 * whatever else happened to answer. Neighbouring 17801 is reserved for
 * `aoss serve`.
 */
const DEV_PORT = 17800

export default defineConfig({
  plugins: [react()],
  server: {
    port: DEV_PORT,
    // Fail loudly instead of drifting to another port: `devUrl` in
    // tauri.conf.json names this one, so a silent fallback would leave the
    // window loading nothing — or a stale server from an earlier run.
    strictPort: true,
    hmr: { port: DEV_PORT }
  },
  build: {
    // A stack trace out of the webview should be readable without shipping
    // the whole toolchain.
    sourcemap: true
  }
})
