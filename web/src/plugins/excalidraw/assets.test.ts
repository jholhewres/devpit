import { afterEach, describe, expect, it } from 'vitest'

import { setExcalidrawAssetPath } from './assets'

describe('setExcalidrawAssetPath', () => {
  afterEach(() => {
    delete window.EXCALIDRAW_ASSET_PATH
  })

  it('points the global at the local bundle, under the app base', () => {
    setExcalidrawAssetPath()
    expect(window.EXCALIDRAW_ASSET_PATH).toBe(`${import.meta.env.BASE_URL}excalidraw/`)
  })

  it('never leaves the package to fall back to esm.sh', () => {
    setExcalidrawAssetPath()
    expect(window.EXCALIDRAW_ASSET_PATH).not.toContain('esm.sh')
  })
})
