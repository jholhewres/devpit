import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join, resolve, sep } from 'node:path'

import { defineConfig, type Plugin } from 'vite'
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

/*
 * Excalidraw loads its hand-drawn fonts at runtime from esm.sh unless told
 * otherwise. The app works offline, so they are served from its own bundle
 * at `<base>excalidraw/fonts/`: read from node_modules in dev, copied into
 * outDir on build — never checked in.
 *
 * Two plugins, split by `apply`: Vitest closes the plugin container with
 * `build.outDir` stubbed, and an unscoped `closeBundle` copied fonts there.
 */
function excalidrawFontsDir(): string {
  const require = createRequire(import.meta.url)
  // `require.resolve('@excalidraw/excalidraw')` lands on `dist/prod/index.js`
  // under plain Node's default export condition; `fonts` sits next to it.
  return join(dirname(require.resolve('@excalidraw/excalidraw')), 'fonts')
}

function excalidrawFontsDev(): Plugin {
  const fontsDir = excalidrawFontsDir()

  return {
    name: 'excalidraw-fonts-dev',
    apply: 'serve',
    configureServer(server) {
      const prefix = `${server.config.base}excalidraw/fonts/`
      server.middlewares.use((req, res, next) => {
        const url = req.url ?? ''
        if (!url.startsWith(prefix)) {
          next()
          return
        }
        const rel = decodeURIComponent(url.slice(prefix.length).split('?')[0]!)
        // Resolved, then checked against its root before it is read — same
        // rule as the backend (AGENTS.md). `resolve` alone is not enough: a
        // `rel` with enough `../` (or an absolute path) can still land
        // outside `fontsDir`, so the result must be checked afterwards.
        const file = resolve(fontsDir, rel)
        if (file !== fontsDir && !file.startsWith(fontsDir + sep)) {
          next()
          return
        }
        if (!existsSync(file)) {
          next()
          return
        }
        res.setHeader('Content-Type', 'font/woff2')
        res.end(readFileSync(file))
      })
    },
  }
}

function excalidrawFontsBuild(): Plugin {
  const fontsDir = excalidrawFontsDir()
  let outDir = 'dist'
  let root = process.cwd()

  return {
    name: 'excalidraw-fonts-build',
    apply: 'build',
    configResolved(config) {
      outDir = config.build.outDir
      root = config.root
    },
    closeBundle() {
      // Xiaolai, the CJK fallback, is 13 MB of the 14. Its faces only claim CJK
      // ranges, so Latin text never asks for it; CJK text finds it missing and
      // the webview falls back to a system font (the CSP keeps esm.sh out).
      cpSync(fontsDir, resolve(root, outDir, 'excalidraw/fonts'), {
        recursive: true,
        filter: (source) => !source.startsWith(join(fontsDir, 'Xiaolai')),
      })
    },
  }
}

/*
 * Tauri embeds all of `frontendDist` in the binary, and the maps were over
 * half of it. They are moved beside `dist` rather than dropped, so a stack
 * trace can still be read against the build that produced it.
 */
function sourcemapsBesideDist(): Plugin {
  let outDir = 'dist'
  let root = process.cwd()

  return {
    name: 'sourcemaps-beside-dist',
    apply: 'build',
    configResolved(config) {
      outDir = config.build.outDir
      root = config.root
    },
    closeBundle() {
      const from = resolve(root, outDir)
      const to = `${from}-sourcemaps`
      rmSync(to, { recursive: true, force: true })
      for (const rel of readdirSync(from, { recursive: true, encoding: 'utf8' })) {
        if (!rel.endsWith('.map')) continue
        mkdirSync(dirname(join(to, rel)), { recursive: true })
        renameSync(join(from, rel), join(to, rel))
      }
    },
  }
}

export default defineConfig({
  plugins: [react(), excalidrawFontsDev(), excalidrawFontsBuild(), sourcemapsBesideDist()],
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
    // the whole toolchain — or the maps: 'hidden' leaves no comment pointing
    // at a file the bundle no longer has.
    sourcemap: 'hidden'
  }
})
