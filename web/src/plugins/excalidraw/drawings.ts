import type { Tab } from '../../shell/strip'

/*
 * What the shell knows about drawings without loading the editor.
 *
 * The menus and the mount table import this file; the canvas stays behind
 * `import()`, so nothing here may reach `@excalidraw/excalidraw`.
 */

export const PLUGIN_ID = 'excalidraw'
export const EXTENSION = '.excalidraw'

/** How long a drawing waits after the last change before it is written. */
export const SAVE_AFTER_MS = 800

/** The list of drawings is one tab, so opening it again lands on it. Named
    after the capability, since that is what the person turned on. */
export const DRAWINGS: Partial<Tab> = { id: 'drawings', title: 'Excalidraw' }

/** One tab per file: the id is the name, so an open drawing is focused, not opened twice. */
export function drawingTab(name: string): Partial<Tab> {
  return { id: `drawing:${name}`, path: name, title: stemOf(name) }
}

export function stemOf(name: string): string {
  return name.endsWith(EXTENSION) ? name.slice(0, -EXTENSION.length) : name
}

/** Past any name a person gives a drawing, and inside the 255 bytes every
    filesystem devpit runs on allows a name. Mirrors `LONGEST_DATA_NAME`. */
const LONGEST_DATA_NAME = 200

/** Names Windows opens as a device, whatever follows the first dot. Mirrors
    `DEVICE_NAMES`. */
const DEVICE_NAMES = [
  'con',
  'prn',
  'aux',
  'nul',
  'com1',
  'com2',
  'com3',
  'com4',
  'com5',
  'com6',
  'com7',
  'com8',
  'com9',
  'lpt1',
  'lpt2',
  'lpt3',
  'lpt4',
  'lpt5',
  'lpt6',
  'lpt7',
  'lpt8',
  'lpt9',
]

/** Separators, what Windows refuses in a name, and the bidi controls that
    draw a name in an order other than the one it is spelled in. Mirrors
    `refused_in_a_name`. */
function refusedInAName(codePoint: number): boolean {
  return (
    (codePoint <= 0x1f || codePoint === 0x7f) || // C0 controls and DEL, as `char::is_control`
    (codePoint >= 0x80 && codePoint <= 0x9f) || // C1 controls, as `char::is_control`
    [0x2f, 0x5c, 0x3a, 0x3c, 0x3e, 0x22, 0x7c, 0x3f, 0x2a].includes(codePoint) || // / \ : < > " | ? *
    (codePoint >= 0x202a && codePoint <= 0x202e) ||
    (codePoint >= 0x2066 && codePoint <= 0x2069)
  )
}

/** Folds ASCII letters only, the alphabet `DEVICE_NAMES` is drawn from — a
    locale-aware `toLowerCase` is not the fold `eq_ignore_ascii_case` uses. */
function asciiLower(letter: string): string {
  const code = letter.codePointAt(0) ?? 0
  return code >= 0x41 && code <= 0x5a ? String.fromCodePoint(code + 0x20) : letter
}

/** The backend's `home::plain_data_name`, so its refusal is said before the
    round trip. Byte length, not string length: the backend counts UTF-8
    bytes. */
export function plainDataName(name: string): boolean {
  const stem = name.split('.')[0] ?? ''
  const stemLower = [...stem.replace(/ +$/, '')].map(asciiLower).join('')
  return (
    name !== '' &&
    new TextEncoder().encode(name).length <= LONGEST_DATA_NAME &&
    !name.startsWith('.') &&
    !name.endsWith('.') &&
    !name.endsWith(' ') &&
    !name.includes('..') &&
    ![...name].some((letter) => refusedInAName(letter.codePointAt(0) ?? 0)) &&
    !DEVICE_NAMES.includes(stemLower)
  )
}

export type Named = { readonly name: string } | { readonly problem: string }

/** The file a typed name becomes, or why it cannot become one. */
export function drawingName(typed: string): Named {
  const stem = stemOf(typed.trim())
  if (stem === '') return { problem: 'Give the drawing a name.' }
  const name = `${stem}${EXTENSION}`
  return plainDataName(name)
    ? { name }
    : {
        problem:
          'A name cannot start or end with a dot or space, hold "..", a separator, or a reserved device name.',
      }
}

/** A new drawing: an empty scene in the shape `drawingFileValid` accepts. */
export const EMPTY_DRAWING = JSON.stringify({
  type: 'excalidraw',
  version: 2,
  source: 'devpit',
  elements: [],
  appState: {},
  files: {},
})
