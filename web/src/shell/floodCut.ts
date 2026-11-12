/*
 * A flood written from where it can still be seen.
 *
 * Once the scrollback is full, every line xterm parses costs a copy of a
 * whole row, and that parse, not the drawing, held a flood at 2 MB/s. Plain
 * text that would only scroll past the scrollback leaves nothing behind but
 * the colour it set, so it is cut out and the colour carried: the terminal
 * ends up exactly as it would have.
 */

const GROUND = 0
const ESCAPE = 1
const ESCAPE_MORE = 2
const CSI = 3
const OSC = 4
/** DCS, SOS, PM and APC: ended by ST only. */
const STRING = 5

/** Where the parser is between two writes. */
export interface Stream {
  state: number
  /** The last byte was 0xC2, which may begin a C1 control. */
  c2: boolean
  /* Which buffers may have a scroll region set, one bit each, and whether
     the alternate one is in use. A line feed below a region scrolls nothing,
     and what a cut line left on a row stays there: nothing is cut then. */
  region: number
  alt: boolean
  /** The CSI being read, which a write may end in the middle of. */
  csi: Csi
}

/* Colour or erase so far; any parameter; a private marker or intermediate;
   a `!` among them; a `?` first; the number being read; a buffer switch. */
interface Csi {
  plain: boolean
  params: boolean
  odd: boolean
  bang: boolean
  query: boolean
  number: number
  swap: boolean
}

export const ground = (): Stream => ({
  state: GROUND,
  c2: false,
  region: 0,
  alt: false,
  csi: { plain: true, params: false, odd: false, bang: false, query: false, number: -1, swap: false },
})

const bit = (stream: Stream): number => (stream.alt ? 2 : 1)
/** The buffers xterm switches with `CSI ? n h` and `l`. */
const SWAPS = new Set([47, 1047, 1049])

/** Line feeds looked back over for a cut, past the ones that must follow it. */
const LOOK_BACK = 256

export interface Cut {
  /** What to write instead of the bytes given. */
  bytes: Uint8Array
  /** Bytes taken out. */
  cut: number
  /** The ring used, for the next call. */
  ring: Int32Array
}

/* Per line feed: where it is, the run of cuttable bytes it is in (-1 for
   none), and whether a CR comes right before it in that run. */
const SLOT = 3

/**
 * The terminal's size, or `null` to cut nothing. `tallest`: the most rows the
 * pane can grow to before xterm parses the write, which comes after the cut.
 */
export type Screen = { rows: number; scrollback: number; tallest?: number } | null

/**
 * Line feeds that must follow a cut: up to a screen of them to bring the
 * cursor down to the last row, then a screen and the scrollback to push out
 * every row written before, with one to spare either side.
 */
const keepFor = (screen: NonNullable<Screen>): number =>
  screen.scrollback + 2 * Math.max(screen.rows, screen.tallest ?? 0) + 2

/**
 * Cuts `bytes`, which start where `stream` left off, at a CR LF followed by
 * enough line feeds of the same plain run (`keepFor`); `stream` moves on to
 * their end either way. A plain run is text, tabs, CR, LF, SGR and
 * erase-in-line: nothing in it moves the cursor up or changes the terminal
 * but its rows and its colour.
 */
export function cutFlood(bytes: Uint8Array, stream: Stream, screen: Screen, ring: Int32Array = new Int32Array(0)): Cut {
  const keep = screen ? keepFor(screen) : 0
  const size = keep + LOOK_BACK
  const at: Int32Array = ring.length >= size * SLOT ? ring : new Int32Array(size * SLOT)
  let feeds = 0
  const feed = (i: number, run: number): void => {
    const slot = (feeds % size) * SLOT
    at[slot] = i
    at[slot + 1] = run
    at[slot + 2] = run >= 0 && i > run + 1 && bytes[i - 1] === 0x0d ? 1 : 0
    feeds += 1
  }
  /* Start of the plain run, or -1 outside one. */
  let run = stream.state === GROUND && !stream.c2 ? 0 : -1
  let { plain, params, odd, bang, query, number, swap } = stream.csi
  const opened = (): void => {
    params = odd = bang = query = swap = false
    number = -1
  }
  for (let i = 0; i < bytes.length; i += 1) {
    const b = bytes[i]!
    const c2 = stream.c2
    stream.c2 = b === 0xc2
    if (c2 && b >= 0x80 && b <= 0x9f) {
      c1(stream, b)
      run = -1
      opened()
      plain = false
      odd = true
      continue
    }
    if (b === 0x18 || b === 0x1a) {
      stream.state = GROUND
      run = i + 1
      continue
    }
    if (b === 0x1b) {
      /* Only an ESC from ground can open a colour; one that cuts a sequence
         short ends the run. */
      plain = stream.state === GROUND
      if (!plain) run = -1
      stream.state = ESCAPE
      continue
    }
    switch (stream.state) {
      case GROUND:
        if (b === 0x0a) feed(i, stream.region & bit(stream) ? -1 : run)
        else if ((b < 0x20 && b !== 0x09 && b !== 0x0d) || b === 0x7f) run = -1
        if (run < 0) run = i + 1
        break
      case ESCAPE:
        if (b === 0x5b) {
          stream.state = CSI
          opened()
        } else if (b === 0x5d) stream.state = OSC
        else if (b === 0x50 || b === 0x58 || b === 0x5e || b === 0x5f) stream.state = STRING
        else if (b >= 0x20 && b <= 0x2f) stream.state = ESCAPE_MORE
        else if (b >= 0x30 && b <= 0x7e) {
          stream.state = GROUND
          if (b === 0x63) {
            stream.region = 0
            stream.alt = false
          }
        } else if (b === 0x0a) feed(i, -1)
        if (stream.state !== CSI) run = stream.state === GROUND ? i + 1 : -1
        break
      case ESCAPE_MORE:
        if (b >= 0x30 && b <= 0x7e) {
          stream.state = GROUND
          run = i + 1
        } else if (b === 0x0a) feed(i, -1)
        break
      case CSI:
        if (b >= 0x40 && b <= 0x7e) {
          stream.state = GROUND
          swap ||= query && SWAPS.has(number)
          if (b === 0x72 && !odd) stream.region = params ? stream.region | bit(stream) : stream.region & ~bit(stream)
          else if (b === 0x70 && bang) stream.region &= ~bit(stream)
          else if ((b === 0x68 || b === 0x6c) && swap) stream.alt = b === 0x68
          /* Colour and erase-in-line stay in the run; anything else ends it. */
          if (!(plain && !odd && (b === 0x6d || b === 0x4b)) || run < 0) run = i + 1
        } else if (b === 0x0a) {
          feed(i, -1)
          plain = false
        } else if (b >= 0x30 && b <= 0x39) {
          params = true
          number = Math.min(99_999, (number < 0 ? 0 : number) * 10 + b - 0x30)
        } else if (b === 0x3b || b === 0x3a) {
          params = true
          swap ||= query && SWAPS.has(number)
          number = -1
        } else {
          query ||= b === 0x3f && !params && !odd
          odd = true
          bang ||= b === 0x21
        }
        break
      case OSC:
        if (b === 0x07) {
          stream.state = GROUND
          run = i + 1
        }
        break
      default:
        break
    }
  }
  stream.csi = { plain, params, odd, bang, query, number, swap }

  /* The last CR LF with `keep` line feeds after it in the same run. */
  for (let n = feeds - keep; keep > 0 && n >= Math.max(0, feeds - size); n -= 1) {
    const slot = (n % size) * SLOT
    const from = at[slot + 1]!
    const last = ((n + keep - 1) % size) * SLOT
    if (at[slot + 2] !== 1 || at[last + 1] !== from) continue
    const cut = at[slot]! - 1
    const carry = colourAt(bytes, from, cut)
    const out = new Uint8Array(from + carry.length + bytes.length - cut)
    out.set(bytes.subarray(0, from), 0)
    out.set(carry, from)
    out.set(bytes.subarray(cut), from + carry.length)
    return { bytes: out, cut: bytes.length - out.length, ring: at }
  }
  return { bytes, cut: 0, ring: at }
}

/** A C1 control, as xterm reads it from its UTF-8. */
function c1(stream: Stream, b: number): void {
  if (b === 0x9b) stream.state = CSI
  else if (b === 0x9d) stream.state = OSC
  else if (b === 0x90 || b === 0x98 || b === 0x9e || b === 0x9f) stream.state = STRING
  else if (b === 0x9c) stream.state = GROUND
}

/**
 * The SGRs in `bytes[from, to)` that still hold at `to`: back to the last
 * reset. Every ESC in a cuttable run starts an SGR or an erase-in-line.
 */
function colourAt(bytes: Uint8Array, from: number, to: number): Uint8Array {
  const kept: Uint8Array[] = []
  let length = 0
  for (let i = to - 1; i >= from; i -= 1) {
    if (bytes[i] !== 0x1b) continue
    let end = i + 2
    while (end < to && !(bytes[end]! >= 0x40 && bytes[end]! <= 0x7e)) end += 1
    if (bytes[end] !== 0x6d) continue
    const one = bytes.subarray(i, end + 1)
    kept.push(one)
    length += one.length
    if (end === i + 2 || (end === i + 3 && bytes[i + 2] === 0x30)) break
  }
  const out = new Uint8Array(length)
  let at = 0
  for (let k = kept.length - 1; k >= 0; k -= 1) {
    out.set(kept[k]!, at)
    at += kept[k]!.length
  }
  return out
}
