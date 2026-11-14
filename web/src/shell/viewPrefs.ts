import { useSyncExternalStore } from 'react'

import type { Press } from './keys'
import { matches } from './shortcuts'

/*
 * How this window draws and takes keys: the chat's send key, sizes, contrast,
 * maths and diffs. Kept per viewer in the webview's storage, like the chat's
 * activity filter. A value missing or unknown reads as its default, which is
 * how devpit behaved before the setting existed.
 */

/* In the order a row draws them. */
export const SIZES = ['small', 'default', 'large', 'larger'] as const
export type Size = (typeof SIZES)[number]
const SIZED = ['default', 'small', 'large', 'larger'] as const

const CHOICES = {
  sendKey: ['enter', 'modEnter'],
  followUp: ['queue', 'steer'],
  textSize: SIZED,
  codeSize: SIZED,
  contrast: ['standard', 'more'],
  latex: [false, true],
  diffLayout: ['each', 'split', 'stacked'],
  diffWhitespace: ['show', 'ignore'],
  diffColours: ['standard', 'safe'],
  diffCollapsed: [true, false],
} as const

export type ViewPrefs = { -readonly [K in keyof typeof CHOICES]: (typeof CHOICES)[K][number] }
export type ViewPref = keyof ViewPrefs

/* The first choice of each is today's behaviour. */
export const DEFAULTS = Object.fromEntries(Object.entries(CHOICES).map(([key, all]) => [key, all[0]])) as ViewPrefs

const KEY = 'devpit.viewPrefs'

function read(): ViewPrefs {
  let raw: unknown = null
  try {
    raw = JSON.parse(localStorage.getItem(KEY) ?? 'null')
  } catch {
    raw = null
  }
  const kept = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {}
  const out: Record<string, unknown> = { ...DEFAULTS }
  for (const [key, all] of Object.entries(CHOICES)) {
    if ((all as readonly unknown[]).includes(kept[key])) out[key] = kept[key]
  }
  return out as ViewPrefs
}

let held: ViewPrefs | null = null
const told = new Set<() => void>()

export const viewPrefs = (): ViewPrefs => (held ??= read())

export function setViewPref<K extends ViewPref>(key: K, value: ViewPrefs[K]): void {
  held = { ...viewPrefs(), [key]: value }
  try {
    localStorage.setItem(KEY, JSON.stringify(held))
  } catch {
    // Storage refused: the choice holds until the window closes.
  }
  applyLooks(held)
  for (const listener of told) listener()
}

/** Forgets the window's copy, so the next read is from storage. For tests. */
export const rereadViewPrefs = (): void => {
  held = null
}

function subscribe(listener: () => void): () => void {
  told.add(listener)
  return () => told.delete(listener)
}

export const useViewPrefs = (): ViewPrefs => useSyncExternalStore(subscribe, viewPrefs)

const SCALE: Record<Size, string> = { small: '0.92', default: '1', large: '1.1', larger: '1.2' }

/** What the stylesheet reads: the size scales and the contrast and diff colour sets. */
export function applyLooks(prefs: ViewPrefs, root: HTMLElement = document.documentElement): void {
  root.style.setProperty('--text-scale', SCALE[prefs.textSize])
  root.style.setProperty('--code-scale', SCALE[prefs.codeSize])
  root.dataset.contrast = prefs.contrast
  root.dataset.diffColours = prefs.diffColours
}

/** Which way a message typed while a turn runs goes: the send key's way, or the other key's. */
export function followUpOf(prefs: Pick<ViewPrefs, 'followUp'>, other: boolean): ViewPrefs['followUp'] {
  if (!other) return prefs.followUp
  return prefs.followUp === 'queue' ? 'steer' : 'queue'
}

/** What a press in the composer does: send, send the other way, or nothing of ours (a new line). */
export function sendKeyOf(press: Press, sendKey: ViewPrefs['sendKey']): 'send' | 'other' | null {
  if (matches('steerNow', press)) return sendKey === 'modEnter' ? 'send' : 'other'
  if (press.key !== 'Enter' || press.shiftKey) return null
  return sendKey === 'enter' ? 'send' : null
}
