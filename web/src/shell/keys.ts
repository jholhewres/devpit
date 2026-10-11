/*
 * A key combination: how one is written, read off a press, and printed.
 *
 * Written once for every platform — `Mod` is ⌘ on a Mac and Ctrl elsewhere —
 * and resolved against the platform only when a press is compared or a label
 * drawn, so one registry serves both.
 */

export const onMacNow = (): boolean => typeof navigator !== 'undefined' && /Mac/.test(navigator.platform)

/** A press, as much of one as matching needs. */
export interface Press {
  readonly key: string
  readonly code?: string
  readonly ctrlKey?: boolean
  readonly metaKey?: boolean
  readonly altKey?: boolean
  readonly shiftKey?: boolean
}

/** A combination resolved for one platform. */
export interface Combo {
  readonly ctrl: boolean
  readonly alt: boolean
  readonly shift: boolean
  readonly meta: boolean
  readonly key: string
}

/** The key a press names, the same on every layout that can type it. */
export function keyOf(press: Pick<Press, 'key' | 'code'>): string {
  const { key, code = '' } = press
  if (key === ' ') return 'Space'
  if (key.length !== 1) return key
  /* A letter that a non-Latin layout or ⌥ turned into something else is still
     the key under the finger: Ctrl+K on a Russian keyboard is Ctrl+K. */
  if (key.charCodeAt(0) > 127) {
    const latin = /^Key([A-Z])$/.exec(code) ?? /^Digit(\d)$/.exec(code)
    if (latin) return latin[1]!.toLowerCase()
  }
  return key.toLowerCase()
}

/* Shift belongs to a letter or a named key. On a symbol the layout already
   spent it: `?` is Shift+/ on one keyboard and Shift+another key on the next. */
const shiftCounts = (key: string): boolean => key.length > 1 || key.toLowerCase() !== key.toUpperCase()

function parts(spec: string): string[] {
  if (spec === '+') return ['+']
  if (spec.endsWith('++')) return [...spec.slice(0, -2).split('+'), '+']
  return spec.split('+')
}

/** A written combination, for this platform. */
export function comboOf(spec: string, onMac: boolean): Combo {
  const all = parts(spec)
  const written = all.pop() ?? ''
  const key = written.length === 1 ? written.toLowerCase() : written
  const has = (name: string): boolean => all.includes(name)
  return {
    ctrl: has('Ctrl') || (!onMac && has('Mod')),
    meta: has('Meta') || (onMac && has('Mod')),
    alt: has('Alt'),
    shift: has('Shift') && shiftCounts(key),
    key,
  }
}

export const sameCombo = (left: Combo, right: Combo): boolean =>
  left.key === right.key && left.ctrl === right.ctrl && left.meta === right.meta && left.alt === right.alt && left.shift === right.shift

/** No Ctrl, Alt or ⌘: text in a field, the program's in a terminal. */
export const bare = (combo: Combo): boolean => !combo.ctrl && !combo.alt && !combo.meta

export function pressMatches(press: Press, combo: Combo): boolean {
  const key = keyOf(press)
  return (
    key === combo.key &&
    Boolean(press.ctrlKey) === combo.ctrl &&
    Boolean(press.metaKey) === combo.meta &&
    Boolean(press.altKey) === combo.alt &&
    (!shiftCounts(key) || Boolean(press.shiftKey) === combo.shift)
  )
}

const HELD = new Set(['Control', 'Meta', 'Alt', 'AltGraph', 'Shift', 'OS', 'Super', 'Hyper', 'CapsLock', 'Fn', 'Dead', 'Unidentified', 'Process'])

/** A press written down, or null while only modifiers are held. */
export function specOf(press: Press, onMac: boolean): string | null {
  if (HELD.has(press.key)) return null
  const key = keyOf(press)
  const primary = onMac ? press.metaKey : press.ctrlKey
  const other = onMac ? press.ctrlKey : press.metaKey
  const written: string[] = []
  if (primary) written.push('Mod')
  if (onMac && other) written.push('Ctrl')
  if (press.altKey) written.push('Alt')
  if (press.shiftKey && shiftCounts(key)) written.push('Shift')
  if (!onMac && other) written.push('Meta')
  // ß upper-cases to two letters, which would read back as a named key.
  written.push(key.toUpperCase().length === 1 ? key.toUpperCase() : key)
  return written.join('+')
}

const NAMED: Readonly<Record<string, readonly [mac: string, other: string]>> = {
  ArrowUp: ['↑', '↑'],
  ArrowDown: ['↓', '↓'],
  ArrowLeft: ['←', '←'],
  ArrowRight: ['→', '→'],
  Enter: ['↵', 'Enter'],
  Escape: ['Esc', 'Esc'],
  Delete: ['Del', 'Del'],
  Backspace: ['⌫', 'Backspace'],
  Tab: ['⇥', 'Tab'],
  Insert: ['Ins', 'Insert'],
}

/** A written combination as a title prints it: ⇧⌘D on a Mac, Ctrl+Shift+D elsewhere. */
export function labelOf(spec: string, onMac: boolean): string {
  const combo = comboOf(spec, onMac)
  const key = NAMED[combo.key]?.[onMac ? 0 : 1] ?? (combo.key.length === 1 ? combo.key.toUpperCase() : combo.key)
  if (onMac) return `${combo.ctrl ? '⌃' : ''}${combo.alt ? '⌥' : ''}${combo.shift ? '⇧' : ''}${combo.meta ? '⌘' : ''}${key}`
  return [combo.ctrl && 'Ctrl', combo.alt && 'Alt', combo.shift && 'Shift', combo.meta && 'Super', key].filter(Boolean).join('+')
}
