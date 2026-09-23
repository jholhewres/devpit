import type { CommandBlock } from '../gen/bindings'

/*
 * How a block's facts are written in its header.
 */

/** How long it took: `320ms`, `4.2s`, `3m 05s`, `1h 02m`. */
export function took(block: CommandBlock, nowMs: number): string {
  const ms = Math.max(0, (block.endedAt ?? nowMs) - (block.startedAt ?? nowMs))
  if (ms < 1000) return `${Math.round(ms)}ms`
  const seconds = ms / 1000
  if (seconds < 60) return `${seconds.toFixed(seconds < 10 ? 1 : 0)}s`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ${String(Math.floor(seconds % 60)).padStart(2, '0')}s`
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, '0')}m`
}

/** A folder as the prompt shows it: home as `~`. */
export function shortPath(path: string | null, home: string | null): string {
  if (!path) return ''
  if (home && (path === home || path.startsWith(`${home}/`))) return `~${path.slice(home.length)}`
  return path
}

/** How it ended, as the header colours it. */
export function outcome(block: CommandBlock): 'running' | 'ok' | 'failed' | 'unknown' {
  if (block.endedAt === null) return 'running'
  if (block.code === null) return 'unknown'
  return block.code === 0 ? 'ok' : 'failed'
}

/** The lines of a block's output a filter keeps: every word, in any order. */
export function filtered(lines: readonly string[], query: string): readonly number[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  if (words.length === 0) return lines.map((_, at) => at)
  return lines.flatMap((line, at) => (words.every((word) => line.toLowerCase().includes(word)) ? [at] : []))
}

/* An address in a line of output: http or https, up to the first space or
   the punctuation that usually closes a sentence around one. */
const URL_IN = /https?:\/\/[^\s<>"'`]+[^\s<>"'`.,;:!?)\]}]/g

/** A piece of output text, split where it names a web address. */
export function linked(text: string): readonly { text: string; url?: string }[] {
  const out: { text: string; url?: string }[] = []
  let at = 0
  for (const match of text.matchAll(URL_IN)) {
    const start = match.index ?? 0
    if (start > at) out.push({ text: text.slice(at, start) })
    out.push({ text: match[0], url: match[0] })
    at = start + match[0].length
  }
  if (at < text.length || out.length === 0) out.push({ text: text.slice(at) })
  return out
}
