/*
 * Excalidraw fetches its fonts from `window.EXCALIDRAW_ASSET_PATH`, esm.sh by
 * default. The app works offline, so this points at the copy `vite.config.ts`
 * serves from the bundle.
 */

declare global {
  interface Window {
    EXCALIDRAW_ASSET_PATH?: string
  }
}

/** Must run before `@excalidraw/excalidraw` is imported. */
export function setExcalidrawAssetPath(): void {
  window.EXCALIDRAW_ASSET_PATH = `${import.meta.env.BASE_URL}excalidraw/`
}
