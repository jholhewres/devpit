import { useEffect, useState, type RefObject } from 'react'

import { droppedPaths } from './pasting'

/*
 * Files dropped on one element, by path — the terminal, the tree, the
 * artifacts. The chat listens on the whole page while it is in front, so a
 * drop taken here stops there: one drop, one place it lands.
 */

const carriesFiles = (event: DragEvent): boolean => event.dataTransfer?.types.includes('Files') ?? false

/** Calls `onPaths` with what is dropped on `at`, and the element under the
 *  pointer; answers whether files hover. */
export function useFileDrop(
  at: RefObject<HTMLElement | null>,
  onPaths: ((paths: readonly string[], under: Element | null) => void) | null,
): boolean {
  const [hovering, setHovering] = useState(false)
  useEffect(() => {
    const element = at.current
    if (!element || !onPaths) return
    const over = (event: DragEvent): void => {
      if (!carriesFiles(event)) return
      event.preventDefault()
      event.stopPropagation()
      setHovering(true)
    }
    const left = (event: DragEvent): void => {
      if (!element.contains(event.relatedTarget as Node | null)) setHovering(false)
    }
    const dropped = (event: DragEvent): void => {
      setHovering(false)
      if (!carriesFiles(event)) return
      event.preventDefault()
      event.stopPropagation()
      const paths = droppedPaths(event.dataTransfer)
      if (paths.length > 0) onPaths(paths, event.target instanceof Element ? event.target : null)
    }
    element.addEventListener('dragover', over)
    element.addEventListener('dragleave', left)
    element.addEventListener('drop', dropped)
    return () => {
      element.removeEventListener('dragover', over)
      element.removeEventListener('dragleave', left)
      element.removeEventListener('drop', dropped)
    }
  }, [at, onPaths])
  return hovering
}

/** A path as one shell word: bare when nothing in it is special, else in
 *  single quotes, inside which a shell reads nothing but the closing quote. */
export function shellWord(path: string): string {
  if (/^[\w@%+=:,./-]+$/.test(path)) return path
  return `'${path.replace(/'/g, `'\\''`)}'`
}

/** The folder a drop on a tree row goes into: the folder itself, a file's
 *  own folder, or the root when it landed on no row. */
export function folderUnder(under: Element | null): string {
  const row = under?.closest<HTMLElement>('[data-path]')
  const path = row?.dataset.path ?? ''
  if (row?.dataset.kind === 'folder') return path
  return path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : ''
}
