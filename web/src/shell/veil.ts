/*
 * The fade the answer arrives under.
 *
 * Text that lands all at once reads as a jump; text that fades in reads as
 * someone writing. So each delta is remembered as a range of the source, and
 * only those ranges are drawn under a fade — the text already on screen when
 * a new delta lands stays exactly where it was.
 *
 * The fade is timed off the stream rather than fixed: a provider sending a
 * word every 80ms and one sending a paragraph every 600ms should both read as
 * writing, and a single duration cannot do both. So the gap between deltas is
 * tracked as a moving average and the fade is three of them, bounded.
 */

const SEED_MS = 160
const MIN_MS = 120
const MAX_MS = 400
/* A pause is a pause, not a signal to slow every later fade to a crawl. */
const GAP_CAP_MS = 1000
/* Past this the oldest are dropped: they have long since finished fading. */
const MAX_CHUNKS = 32

export interface Chunk {
  readonly start: number
  readonly end: number
  readonly at: number
  readonly ms: number
}

export interface Veil {
  previous: string
  ema: number
  last: number | null
  chunks: Chunk[]
}

/* Text already present when a message mounts is the baseline and never fades:
   re-opening a finished conversation should not replay it. */
export const opened = (text = ''): Veil => ({ previous: text, ema: SEED_MS, last: null, chunks: [] })

const fadeMs = (ema: number): number => Math.min(MAX_MS, Math.max(MIN_MS, ema * 3))

/* Several chunks fading at once would stack into a visible lag, so each extra
   one past the second shortens all of them. */
const boost = (count: number): number => 1 + 0.3 * Math.max(0, count - 2)

const shared = (left: string, right: string): number => {
  const limit = Math.min(left.length, right.length)
  let at = 0
  while (at < limit && left.charCodeAt(at) === right.charCodeAt(at)) at += 1
  return at
}

/* Register what just arrived and forget what has finished fading. Returns the
   ranges still under the fade, in source coordinates. */
export function advanced(veil: Veil, text: string, streaming: boolean, now: number): readonly Chunk[] {
  if (!streaming) {
    veil.previous = text
    veil.chunks = []
    veil.last = null
    return []
  }

  if (text !== veil.previous) {
    const prefix = shared(veil.previous, text)
    /* A rewritten tail — a provider correcting itself — invalidates the part
       of a chunk that no longer exists rather than the whole chunk. */
    veil.chunks = veil.chunks.flatMap((chunk) => {
      const end = Math.min(chunk.end, prefix)
      return chunk.start < end ? [{ ...chunk, end }] : []
    })
    if (text.length > prefix) {
      if (veil.last !== null) {
        const gap = Math.min(Math.max(0, now - veil.last), GAP_CAP_MS)
        veil.ema = veil.ema * 0.7 + gap * 0.3
      }
      veil.last = now
      veil.chunks.push({ start: prefix, end: text.length, at: now, ms: fadeMs(veil.ema) })
      if (veil.chunks.length > MAX_CHUNKS) veil.chunks.splice(0, veil.chunks.length - MAX_CHUNKS)
    }
    veil.previous = text
  }

  const rate = boost(veil.chunks.length)
  veil.chunks = veil.chunks.filter((chunk) => (now - chunk.at) * rate < chunk.ms)
  return veil.chunks
}

/* The animation as CSS variables. The delay is negative so a chunk that
   started before this render resumes where it is, instead of restarting every
   time React draws. */
export function styleOf(chunk: Chunk, now: number, count: number): React.CSSProperties {
  const duration = chunk.ms / boost(count)
  const elapsed = Math.min(Math.max(0, now - chunk.at), duration)
  return {
    ['--veil-ms' as string]: `${duration}ms`,
    ['--veil-delay' as string]: `-${elapsed}ms`,
  }
}

export interface Slice {
  readonly text: string
  readonly chunk: Chunk | null
}

/* One rendered run of text, cut where the fading ranges begin and end.

   `from` is where this run starts in the source. The caller finds that by
   searching forward from a cursor rather than by asking the parser: the
   parser strips markers as it goes, so its offsets would have to be threaded
   through every branch, and a search that walks forward once costs the same
   and cannot drift. */
export function cut(text: string, from: number, chunks: readonly Chunk[]): readonly Slice[] {
  if (!chunks.length || !text) return [{ text, chunk: null }]
  const to = from + text.length
  const cuts = new Set([from, to])
  let touched = false
  for (const chunk of chunks) {
    const start = Math.max(from, chunk.start)
    const end = Math.min(to, chunk.end)
    if (start < end) {
      touched = true
      cuts.add(start)
      cuts.add(end)
    }
  }
  if (!touched) return [{ text, chunk: null }]

  const edges = [...cuts].sort((left, right) => left - right)
  return edges.slice(0, -1).map((start, at) => {
    const end = edges[at + 1]!
    return {
      text: text.slice(start - from, end - from),
      chunk: chunks.find((candidate) => candidate.start <= start && end <= candidate.end) ?? null,
    }
  })
}

/* Where a rendered run sits in the source.

   Every run is a verbatim slice of it — the parser only ever drops markers,
   never rewrites text — so a forward search from the last run's end lands on
   the right one. `-1` when it does not, which only says this run will not
   fade. */
export function locate(source: string, text: string, cursor: number): number {
  if (!text) return -1
  return source.indexOf(text, cursor)
}
