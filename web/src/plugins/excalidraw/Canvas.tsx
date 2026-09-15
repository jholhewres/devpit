import { useEffect, useMemo, useRef, useState } from 'react'

import '@excalidraw/excalidraw/index.css'

import { drawingFileValid, pasteAllowed, withoutFrames } from './drawingFile'
import { setExcalidrawAssetPath } from './assets'

type ExcalidrawWidget = typeof import('@excalidraw/excalidraw').Excalidraw
type Restore = typeof import('@excalidraw/excalidraw').restore
type SerializeAsJSON = typeof import('@excalidraw/excalidraw').serializeAsJSON

export interface CanvasProps {
  /** The `.excalidraw` file's current text, or `''` for a new drawing. */
  initialText: string
  theme: 'light' | 'dark'
  /** `serializeAsJSON(elements, appState, files, 'local')`, on every change. */
  onChange: (serialized: string) => void
}

/*
 * The Excalidraw canvas. Import this module lazily: its CSS rides with it,
 * and the package itself is pulled in on mount (as `shell/Diagram.tsx` does
 * for mermaid) because it is a megabyte and cannot load under jsdom.
 */
export function Canvas({ initialText, theme, onChange }: CanvasProps): React.JSX.Element {
  const [Widget, setWidget] = useState<ExcalidrawWidget | null>(null)
  const restoreRef = useRef<Restore | null>(null)
  const serializeRef = useRef<SerializeAsJSON | null>(null)
  // Excalidraw reads `initialData` once, on mount; restoring on every render
  // would parse the whole file each time a stroke re-renders the parent.
  const initialData = useMemo(() => {
    if (!Widget || !drawingFileValid(initialText)) return undefined
    const restored = restoreRef.current!(JSON.parse(initialText), null, null)
    // The file check already refuses frames; this holds if that check ever loosens.
    return { ...restored, elements: withoutFrames(restored.elements) }
  }, [Widget, initialText])

  useEffect(() => {
    let dropped = false
    setExcalidrawAssetPath()
    void (async () => {
      const mod = await import('@excalidraw/excalidraw')
      if (dropped) return
      restoreRef.current = mod.restore
      serializeRef.current = mod.serializeAsJSON
      setWidget(() => mod.Excalidraw)
    })()
    return () => {
      dropped = true
    }
  }, [])

  if (!Widget) return <div className="plg-excalidraw__loading" />

  return (
    <Widget
      theme={theme}
      initialData={initialData}
      // Embeddables never validate, so none loads a URL. Iframe elements render
      // regardless, so they are dropped on load and refused on paste.
      validateEmbeddable={false}
      renderEmbeddable={() => null}
      onPaste={pasteAllowed}
      // Links do not open: the desktop opener takes project paths, not web addresses.
      onLinkOpen={(_element, event) => event.preventDefault()}
      // Opening another file here would replace this drawing and autosave over it.
      UIOptions={{ canvasActions: { loadScene: false } }}
      // The toolbar's Web Embed and a drop can still add a frame; never saving one
      // keeps the file openable, since the file check refuses frames.
      onChange={(elements, appState, files) => {
        onChange(serializeRef.current!(withoutFrames(elements), appState, files, 'local'))
      }}
    />
  )
}
