import type { Tab } from '../../shell/strip'
import { fileTab, stemOf as stem, type FileKind } from '../pluginFile'

/*
 * What the shell knows about data files without loading the graph.
 *
 * The menus and the mount table import this; jsoncrack stays behind
 * `import()`, so nothing here may reach it.
 */

export const PLUGIN_ID = 'data'

export const DATA: FileKind = {
  pluginId: PLUGIN_ID,
  extension: '.json',
  noun: 'data file',
  empty: '{\n  "name": "example",\n  "items": []\n}\n',
}

/** How many nodes the graph will draw before it says the file is too big.
 *
 * The package runs smoothly to a few hundred; past that the browser is doing
 * layout work nobody asked for on a file they wanted to look at. Its own
 * overlay says so, which beats a blank canvas or a frozen window. */
export const MOST_NODES = 500

/** The list is one tab, so opening it again lands on it. */
export const DATA_FILES: Partial<Tab> = { id: 'data', title: 'Data' }

export const dataTab = (name: string): Partial<Tab> => fileTab(DATA, name)
export const stemOf = (name: string): string => stem(DATA, name)

/** Whether this file is YAML rather than JSON, by its name. */
export const isYaml = (name: string): boolean => name.endsWith('.yaml') || name.endsWith('.yml')

/**
 * The JSON a file's text is, or why it is not any.
 *
 * YAML comes through `js-yaml` and JSON through `JSON.parse`, and both end as
 * the same thing: the graph takes one shape. A file halfway through being
 * typed is not an error worth a red screen — the reason is said, and the
 * previous graph stays on screen until it parses again.
 */
export async function asJson(text: string, yaml: boolean): Promise<{ json: unknown } | { problem: string }> {
  try {
    if (!yaml) return { json: JSON.parse(text) }
    const { load } = await import('js-yaml')
    return { json: load(text) }
  } catch (thrown) {
    return { problem: (thrown as Error).message }
  }
}
