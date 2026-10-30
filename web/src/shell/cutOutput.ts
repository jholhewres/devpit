import { createContext } from 'react'

/*
 * A tool's output the window was sent cut: long text as its head and tail,
 * an image by its size. The transcript keeps it whole; opening the row asks
 * for it by the call it answers.
 */

/* The line the backend writes where it left text out (`crates/rpc/src/cut.rs`):
   on a line of its own, between the head and the tail. */
const CUT_LINE = /\n\u2026 \[cut: \d+ bytes in all\]\n/

/* The output's content blocks, or `null` when it is not an array of them:
   every item an object with a string `type`. */
function blocksOf(output: string): readonly Record<string, unknown>[] | null {
  try {
    const blocks = JSON.parse(output) as unknown
    return Array.isArray(blocks) && blocks.every((block) => block && typeof block === 'object' && typeof block.type === 'string')
      ? blocks
      : null
  } catch {
    return null
  }
}

/* An image block that still carries its data. */
const carriesImage = (block: Record<string, unknown>): boolean =>
  Boolean(block.source) && typeof (block.source as { data?: unknown }).data === 'string'

/* Whether the window got this output cut: the backend's line in a text, or an
   image named instead of carried. An output that merely mentions either is
   not. */
export function isCut(output: string): boolean {
  if (CUT_LINE.test(output)) return true
  const blocks = blocksOf(output) ?? []
  return blocks.some((block) => (block.type === 'image' && block.cut === true) || (typeof block.text === 'string' && CUT_LINE.test(block.text)))
}

/* Where a row's message is, to ask for its whole output. */
export interface Whole {
  readonly projectId: string
  readonly conversationId: string
}

export const WholeContext = createContext<Whole | null>(null)

/* An image block the backend named instead of carrying. */
export interface NamedImage {
  readonly mime: string | null
  readonly bytes: number
}

export function namedImages(output: string): readonly NamedImage[] {
  return (blocksOf(output) ?? []).flatMap((block) =>
    block.type === 'image' && block.cut === true
      ? [{ mime: typeof block.mime === 'string' ? block.mime : null, bytes: Number(block.bytes) || 0 }]
      : [],
  )
}

/* Images of a whole output, as data URLs, in order. */
export function imagesOf(output: string): readonly string[] {
  return (blocksOf(output) ?? []).filter(carriesImage).map((block) => {
    const source = block.source as { data: string; media_type?: unknown }
    return `data:${typeof source.media_type === 'string' ? source.media_type : 'image/png'};base64,${source.data}`
  })
}

/* A whole output beside its images: its text blocks, the other blocks as they
   are, or all of it when it is not content blocks. */
export function textOf(output: string): string {
  const blocks = blocksOf(output)
  if (!blocks) return output
  const rest = blocks.filter((block) => !carriesImage(block))
  return rest.every((block) => typeof block.text === 'string')
    ? rest.map((block) => block.text).join('\n')
    : JSON.stringify(rest, null, 2)
}
