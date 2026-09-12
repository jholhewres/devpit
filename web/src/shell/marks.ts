/*
 * A terminal as a sequence of commands rather than a wall of bytes.
 *
 * The shell reports where a prompt begins, where a command begins, and what it
 * exited with — see `crates/pty/src/shell.rs` for how it is made to. Those
 * three arrivals are enough to know that the eleven hundred lines on screen
 * are four commands, that the third failed, and where each one started.
 *
 * The rules are here rather than in the component because they are a state
 * machine over an event stream, which is the kind of thing that is either
 * tested or wrong.
 */

/** What the pane reported. The wire spelling of `Happening.what`. */
export type Said = 'prompt' | 'running' | 'finished' | 'cwd' | 'title' | 'clipboard'

export interface Block {
  /** The row the prompt was drawn on, as the terminal counted rows then. */
  readonly at: number
  /** Absent while it runs; a number once it ends. */
  readonly code: number | null
  /** True between the command starting and it ending. */
  readonly running: boolean
}

export interface Marks {
  readonly blocks: readonly Block[]
}

export const noMarks: Marks = { blocks: [] }

/*
 * A missing code is not a zero.
 *
 * Some shells report `D` with no code for an empty line — you pressed Enter at
 * a bare prompt. Painting that green would be claiming a command succeeded
 * when none ran.
 */
const codeOf = (detail: string | null | undefined): number | null => {
  if (detail === null || detail === undefined || detail.trim() === '') return null
  const code = Number(detail)
  return Number.isInteger(code) ? code : null
}

/**
 * The next state, given what just arrived and which row the terminal is on.
 *
 * `prompt` opens a block. `running` marks the open one as started — and only
 * the open one: a `C` with no prompt before it is a shell that reported half
 * its story, and inventing a block for it would put a mark on a row nobody
 * typed at. `finished` closes it.
 */
export function marked(marks: Marks, said: Said, detail: string | null, row: number): Marks {
  const blocks = marks.blocks
  const last = blocks[blocks.length - 1]

  switch (said) {
    case 'prompt':
      /* Two prompts with no command between them is one prompt redrawn — a
         resize, a Ctrl-C, a widget repainting the line. Keep the block and
         move it to where the prompt is now. */
      if (last && !last.running && last.code === null) {
        return { blocks: [...blocks.slice(0, -1), { ...last, at: row }] }
      }
      return { blocks: [...blocks, { at: row, code: null, running: false }] }

    case 'running':
      if (!last || last.running || last.code !== null) return marks
      return { blocks: [...blocks.slice(0, -1), { ...last, running: true }] }

    case 'finished':
      if (!last || !last.running) return marks
      return {
        blocks: [...blocks.slice(0, -1), { ...last, running: false, code: codeOf(detail) }],
      }

    default:
      return marks
  }
}

/** How the mark on a row should read. */
export type Tone = 'running' | 'ok' | 'failed' | 'none'

export function toneOf(block: Block): Tone {
  if (block.running) return 'running'
  if (block.code === null) return 'none'
  return block.code === 0 ? 'ok' : 'failed'
}

/**
 * The row to jump to, going one command back or forward from `from`.
 *
 * `null` when there is nowhere to go, so the caller leaves the view alone
 * rather than scrolling to the top and calling it an answer.
 */
export function jump(marks: Marks, from: number, direction: 'back' | 'forward'): number | null {
  const rows = marks.blocks.map((block) => block.at)
  if (direction === 'back') {
    const before = rows.filter((row) => row < from)
    return before.length ? before[before.length - 1]! : null
  }
  const after = rows.filter((row) => row > from)
  return after.length ? after[0]! : null
}
