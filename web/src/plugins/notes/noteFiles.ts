import type { Tab } from '../../shell/strip'
import { fileTab, stemOf as stem, type FileKind } from '../pluginFile'

/*
 * What the shell knows about notes without loading the editor.
 *
 * The menus and the mount table import this; the editor stays behind
 * `import()`, so nothing here may reach it.
 */

export const PLUGIN_ID = 'notes'

export const NOTES_KIND: FileKind = {
  pluginId: PLUGIN_ID,
  extension: '.md',
  noun: 'note',
  /** Empty, because an empty note is a blank page and reads as one. A
   *  diagram needed a starting line; prose does not. */
  empty: '',
}

/** The list is one tab, so opening it again lands on it. */
export const NOTES: Partial<Tab> = { id: 'notes', title: 'Notes' }

export const noteTab = (name: string): Partial<Tab> => fileTab(NOTES_KIND, name)
export const stemOf = (name: string): string => stem(NOTES_KIND, name)
