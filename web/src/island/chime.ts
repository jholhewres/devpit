import type { Doing } from '../gen/bindings'

/*
 * The island's sounds: a few notes, synthesised, so nothing is shipped and
 * nothing is borrowed.
 *
 * Only for what is worth turning your head for — a session stopping on you,
 * asking, finishing, failing. A sound per tool call would be a sound
 * somebody switches off in the first minute, and then the one that mattered
 * goes unheard too.
 */

export type Cue = 'waiting' | 'asking' | 'done' | 'failed'

interface Note {
  readonly at: number
  readonly hz: number
  readonly lasts: number
}

/** Rising for "come back", falling for "it went wrong". */
const TUNES: Readonly<Record<Cue, { readonly wave: OscillatorType; readonly notes: readonly Note[] }>> = {
  waiting: { wave: 'sine', notes: [{ at: 0, hz: 659.25, lasts: 0.22 }, { at: 0.12, hz: 880, lasts: 0.34 }] },
  asking: {
    wave: 'triangle',
    notes: [
      { at: 0, hz: 880, lasts: 0.14 },
      { at: 0.1, hz: 1108.73, lasts: 0.14 },
      { at: 0.2, hz: 1318.51, lasts: 0.3 },
    ],
  },
  done: { wave: 'sine', notes: [{ at: 0, hz: 1046.5, lasts: 0.12 }, { at: 0.08, hz: 1567.98, lasts: 0.3 }] },
  failed: { wave: 'triangle', notes: [{ at: 0, hz: 440, lasts: 0.18 }, { at: 0.14, hz: 329.63, lasts: 0.36 }] },
}

/** The sound a session's change makes, if any. */
export function cueOf(was: Doing | undefined, now: Doing): Cue | null {
  if (was === now) return null
  if (now === 'waiting') return 'waiting'
  if (now === 'failed') return 'failed'
  if (now === 'done' && was === 'working') return 'done'
  return null
}

const KEY = 'devpit.island.sound'

/** Whether the island makes sounds: on unless somebody turned it off here. */
export function soundOn(): boolean {
  try {
    return window.localStorage.getItem(KEY) !== 'off'
  } catch {
    return true
  }
}

export function setSoundOn(on: boolean): void {
  try {
    window.localStorage.setItem(KEY, on ? 'on' : 'off')
  } catch {
    /* Not kept: it still applies until the window closes. */
  }
}

let context: AudioContext | null = null
let resting: ReturnType<typeof setTimeout> | undefined

/** How long after a cue ends the audio is let go: a context left running
 *  keeps an audio thread awake for nothing. */
export const REST_AFTER_MS = 1_500

/** Plays a cue. Silent, never throwing, wherever audio is not there. */
export function play(cue: Cue): void {
  try {
    context ??= new AudioContext()
    if (context.state === 'suspended') void context.resume().catch(() => undefined)
    const start = context.currentTime + 0.02
    const { wave, notes } = TUNES[cue]
    const ends = Math.max(...notes.map((note) => note.at + note.lasts)) + 0.05
    clearTimeout(resting)
    const going = context
    resting = setTimeout(() => void going.suspend().catch(() => undefined), ends * 1000 + REST_AFTER_MS)
    for (const note of notes) {
      const tone = context.createOscillator()
      const level = context.createGain()
      tone.type = wave
      tone.frequency.value = note.hz
      level.gain.setValueAtTime(0.0001, start + note.at)
      level.gain.exponentialRampToValueAtTime(0.12, start + note.at + 0.015)
      level.gain.exponentialRampToValueAtTime(0.0001, start + note.at + note.lasts)
      tone.connect(level).connect(context.destination)
      tone.start(start + note.at)
      tone.stop(start + note.at + note.lasts + 0.05)
    }
  } catch {
    /* No audio here: the island still shows it. */
  }
}
