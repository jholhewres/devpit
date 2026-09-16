import type { Tab } from '../../shell/strip'
import {
  fileName,
  fileTab,
  stemOf as stem,
  type FileKind,
  type Named,
} from '../pluginFile'

/*
 * What the shell knows about drawings without loading the editor.
 *
 * The menus and the mount table import this file; the canvas stays behind
 * `import()`, so nothing here may reach `@excalidraw/excalidraw`.
 */

export const PLUGIN_ID = 'excalidraw'
export const EXTENSION = '.excalidraw'

/** What the shared base needs to know about a drawing. */
export const DRAWING: FileKind = {
  pluginId: PLUGIN_ID,
  extension: EXTENSION,
  noun: 'drawing',
  /** An empty scene in the shape `drawingFileValid` accepts. */
  empty: JSON.stringify({
    type: 'excalidraw',
    version: 2,
    source: 'devpit',
    elements: [],
    appState: {},
    files: {},
  }),
}

export const EMPTY_DRAWING = DRAWING.empty

/** How long a drawing waits after the last change before it is written. */
export { SAVE_AFTER_MS } from '../pluginFile'

/** The list of drawings is one tab, so opening it again lands on it. Named
    after the capability, since that is what the person turned on. */
export const DRAWINGS: Partial<Tab> = { id: 'drawings', title: 'Excalidraw' }

export const drawingTab = (name: string): Partial<Tab> => fileTab(DRAWING, name)
export const stemOf = (name: string): string => stem(DRAWING, name)
export const drawingName = (typed: string): Named => fileName(DRAWING, typed)
export { plainDataName } from '../pluginFile'
export type { Named }
