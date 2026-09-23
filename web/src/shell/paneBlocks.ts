import type { BlockChanged, CommandBlock, Happening, PaneBlocks } from '../gen/bindings'

/*
 * What a terminal knows about its commands: the blocks it has kept, whether
 * its shell marks its prompts at all, whether it is sitting at one, and where.
 *
 * A reducer, so the rules are called by a test instead of being restated by
 * one. Three sources feed it: the pane's blocks as asked for, a block starting
 * or ending (`terminal:block`), and the pane's own happenings — a prompt, a
 * command running, a folder.
 */

export interface BlockState {
  readonly blocks: readonly CommandBlock[]
  readonly integrated: boolean
  readonly atPrompt: boolean
  readonly cwd: string | null
}

export const EMPTY: BlockState = { blocks: [], integrated: false, atPrompt: false, cwd: null }

/** The kept blocks, as asked for. */
export const loaded = (answer: PaneBlocks): BlockState => ({
  blocks: answer.blocks,
  integrated: answer.integrated,
  atPrompt: answer.atPrompt,
  cwd: answer.cwd,
})

/* The most blocks drawn at once. The backend keeps more; a list this long is
   already more than anyone scrolls through. */
export const MOST_SHOWN = 300

/** A block that started, changed or ended: put in its place, or added. */
export function changed(state: BlockState, change: BlockChanged): BlockState {
  const block = change.block
  const at = state.blocks.findIndex((one) => one.id === block.id)
  const blocks = at >= 0 ? state.blocks.map((one, i) => (i === at ? block : one)) : [...state.blocks, block].slice(-MOST_SHOWN)
  return {
    ...state,
    blocks,
    integrated: true,
    atPrompt: block.endedAt === null ? false : state.atPrompt,
    cwd: block.cwd ?? state.cwd,
  }
}

/** What the pane said about itself — the same state when it changes nothing,
 *  since a shell repeats its prompt and its folder far more often than
 *  either moves, and a new state is a new render of the whole block list. */
export function happened(state: BlockState, happening: Happening): BlockState {
  switch (happening.what) {
    case 'prompt':
      return state.integrated && state.atPrompt ? state : { ...state, integrated: true, atPrompt: true }
    case 'running':
      return state.atPrompt ? { ...state, atPrompt: false } : state
    case 'cwd':
      return happening.detail && happening.detail !== state.cwd ? { ...state, cwd: happening.detail } : state
    default:
      return state
  }
}

/** The block still running, if the last one has not ended. */
export function runningOf(state: BlockState): CommandBlock | null {
  const last = state.blocks[state.blocks.length - 1]
  return last && last.endedAt === null ? last : null
}

/**
 * Where Alt+↑ or Alt+↓ goes from the block last jumped to: the bookmarked
 * block before or after it, or simply the block before or after when none is
 * bookmarked. `from` null is the bottom of the list, where the next line is
 * typed; going down past the last stop answers null, back to it. Going up past
 * the first stays there.
 */
export function jumpTarget(blocks: readonly CommandBlock[], from: number | null, by: -1 | 1): number | null {
  const marked = blocks.filter((one) => one.bookmarked)
  const stops = marked.length > 0 ? marked : blocks
  const at = from === null ? -1 : blocks.findIndex((one) => one.id === from)
  const index = (one: CommandBlock): number => blocks.indexOf(one)
  if (by < 0) {
    const before = at < 0 ? stops : stops.filter((one) => index(one) < at)
    return before[before.length - 1]?.id ?? (at < 0 ? null : from)
  }
  if (at < 0) return null
  return stops.find((one) => index(one) > at)?.id ?? null
}

/** The finished blocks, oldest first. */
export const finished = (state: BlockState): readonly CommandBlock[] => state.blocks.filter((one) => one.endedAt !== null)

/**
 * How the terminal should be drawn now.
 *
 *  - `classic`: the shell does not mark its prompts, so there are no blocks
 *    to draw — the terminal as it always was.
 *  - `full`: a command took the whole screen — an editor, an agent's TUI —
 *    and gets it.
 *  - `running`: a command is running; its blocks above, it live below.
 *  - `idle`: at the prompt; the blocks, and the input where the next line is
 *    typed.
 */
export type Mode = 'classic' | 'full' | 'running' | 'idle'

export function modeOf(state: BlockState, wanted: boolean): Mode {
  if (!wanted || !state.integrated) return 'classic'
  const running = runningOf(state)
  if (running?.interactive) return 'full'
  if (running || !state.atPrompt) return 'running'
  return 'idle'
}
