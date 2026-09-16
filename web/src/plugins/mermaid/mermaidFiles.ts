import type { Tab } from '../../shell/strip'
import { fileTab, stemOf as stem, type FileKind } from '../pluginFile'

/*
 * What the shell knows about diagrams without loading the editor or mermaid.
 *
 * The menus and the mount table import this; both of those stay behind
 * `import()`, so nothing here may reach either.
 */

export const PLUGIN_ID = 'mermaid'

export const MERMAID: FileKind = {
  pluginId: PLUGIN_ID,
  extension: '.mmd',
  noun: 'diagram',
  /** A diagram that draws something the moment it is created: an empty file
   *  renders as a parse error, which reads as broken rather than new. */
  empty: 'flowchart LR\n  A[Start] --> B[Next]\n',
}

/** The list is one tab, so opening it again lands on it. */
export const DIAGRAMS: Partial<Tab> = { id: 'diagrams', title: 'Mermaid' }

export const diagramTab = (name: string): Partial<Tab> => fileTab(MERMAID, name)
export const stemOf = (name: string): string => stem(MERMAID, name)
