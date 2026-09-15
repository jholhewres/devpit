/*
 * The one shape of a `.excalidraw` file the canvas trusts.
 *
 * `serializeAsJSON(elements, appState, files, 'local')` always writes
 * `type: "excalidraw"` and `version: 2`. Anything
 * else — malformed JSON, a different type, a future version, an iframe or embed — is
 * rejected before it reaches the canvas, rather than handed to `restore` and hoping.
 */
export function drawingFileValid(text: string): boolean {
  let parsed: unknown
  try {
    parsed = JSON.parse(text)
  } catch {
    return false
  }
  if (typeof parsed !== 'object' || parsed === null) return false
  const data = parsed as Record<string, unknown>
  return (
    data.type === 'excalidraw' &&
    data.version === 2 &&
    Array.isArray(data.elements) &&
    !data.elements.some(unsafeElement)
  )
}

/*
 * Excalidraw 0.18 renders every `iframe` element, with scripts allowed, from
 * HTML the element itself carries, in the webview that holds the terminal IPC;
 * an `embeddable` loads a URL. Neither is kept, whoever wrote the file.
 */
export function unsafeElement(element: unknown): boolean {
  const type = (element as { type?: unknown } | null)?.type
  return type === 'iframe' || type === 'embeddable'
}

export function withoutFrames<T>(elements: readonly T[]): T[] {
  return elements.filter((element) => !unsafeElement(element))
}

/** For `onPaste`: `false` stops Excalidraw before the pasted elements reach the scene. */
export function pasteAllowed(data: { elements?: readonly unknown[] }): boolean {
  return !data.elements?.some(unsafeElement)
}
