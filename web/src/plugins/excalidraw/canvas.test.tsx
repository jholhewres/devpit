import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { Canvas } from './Canvas'

/* The real package cannot load under jsdom, and does not need to: the
   question is what the global said at the moment the package was loaded,
   and what the canvas was handed. */
const loaded = { assetPath: 'never loaded' as string | undefined }
let given: {
  initialData?: { elements: { id: string }[] }
  onPaste?: (data: { elements?: unknown[] }, event: null) => boolean
  onChange?: (elements: { id: string; type: string }[], appState: object, files: object) => void
  UIOptions?: { canvasActions?: { loadScene?: boolean } }
} = {}

vi.mock('@excalidraw/excalidraw', () => {
  loaded.assetPath = window.EXCALIDRAW_ASSET_PATH
  return {
    Excalidraw: (props: typeof given) => {
      given = props
      return <div data-testid="excalidraw" />
    },
    restore: (data: unknown) => data,
    /* The ids it was asked to write, so a test can see what reached the file. */
    serializeAsJSON: (elements: { id: string }[]) => JSON.stringify(elements.map((element) => element.id)),
  }
})
vi.mock('@excalidraw/excalidraw/index.css', () => ({}))
/* The file check refuses frames first; letting every file through tests the canvas's own filter. */
vi.mock('./drawingFile', async (original) => ({
  ...(await original<typeof import('./drawingFile')>()),
  drawingFileValid: (text: string) => text !== '',
}))

afterEach(() => {
  cleanup()
  delete window.EXCALIDRAW_ASSET_PATH
  given = {}
})

describe('mounting the canvas', () => {
  it('points the fonts at the bundle before the package is loaded', async () => {
    render(<Canvas initialText="" theme="light" onChange={() => {}} />)
    await screen.findByTestId('excalidraw')
    expect(loaded.assetPath).toBe(`${import.meta.env.BASE_URL}excalidraw/`)
  })
})

describe('frames never reach the canvas', () => {
  it('drops iframe and embeddable elements from the file before the canvas starts', async () => {
    const text = JSON.stringify({
      type: 'excalidraw',
      version: 2,
      elements: [
        { id: 'r', type: 'rectangle' },
        { id: 'f', type: 'iframe', customData: { generationData: { status: 'done', html: '<script></script>' } } },
        { id: 'e', type: 'embeddable' },
      ],
    })
    render(<Canvas initialText={text} theme="light" onChange={() => {}} />)
    await screen.findByTestId('excalidraw')
    expect(given.initialData!.elements.map((element) => element.id)).toEqual(['r'])
  })

  it('refuses a paste that carries an iframe or an embeddable', async () => {
    render(<Canvas initialText="" theme="light" onChange={() => {}} />)
    await screen.findByTestId('excalidraw')
    expect(given.onPaste!({ elements: [{ type: 'iframe' }] }, null)).toBe(false)
    expect(given.onPaste!({ elements: [{ type: 'embeddable' }] }, null)).toBe(false)
    expect(given.onPaste!({ elements: [{ type: 'rectangle' }] }, null)).toBe(true)
  })

  it('never writes an iframe or embeddable, so a drawing stays openable after Web Embed', async () => {
    const written: string[] = []
    render(<Canvas initialText="" theme="light" onChange={(text) => written.push(text)} />)
    await screen.findByTestId('excalidraw')
    given.onChange!(
      [
        { id: 'r', type: 'rectangle' },
        { id: 'e', type: 'embeddable' },
        { id: 'f', type: 'iframe' },
      ],
      {},
      {},
    )
    expect(written).toEqual(['["r"]'])
  })

  it('offers no way to open another file over this drawing', async () => {
    render(<Canvas initialText="" theme="light" onChange={() => {}} />)
    await screen.findByTestId('excalidraw')
    expect(given.UIOptions?.canvasActions?.loadScene).toBe(false)
  })
})
