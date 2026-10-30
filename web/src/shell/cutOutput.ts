import { createContext } from 'react'

/*
 * A tool's output the window was sent cut: long text as its head and tail,
 * an image by its size. The transcript keeps it whole; opening the row asks
 * for it by the call it answers.
 */

/* The line the backend writes where it left text out (`crates/rpc/src/cut.rs`):
   on a line of its own, between the head and the tail. */
const CUT_LINE = /\n\u2026 \[cut: \d+ bytes in all\]\n/

/* Whether the window got this output cut: the backend's line in a text, or an
   image named instead of carried. An output that merely mentions either is
   not. */
export function isCut(output: string): boolean {
  if (CUT_LINE.test(output)) return true
  if (namedImages(output).length > 0) return true
  try {
    const blocks = JSON.parse(output) as unknown
    return Array.isArray(blocks) && blocks.some((block) => block && typeof block.text === 'string' && CUT_LINE.test(block.text))
  } catch {
    return false
  }
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
  try {
    const blocks = JSON.parse(output) as unknown
    if (!Array.isArray(blocks)) return []
    return blocks.flatMap((block) =>
      block && typeof block === 'object' && block.type === 'image' && block.cut === true
        ? [{ mime: typeof block.mime === 'string' ? block.mime : null, bytes: Number(block.bytes) || 0 }]
        : [],
    )
  } catch {
    return []
  }
}

/* Images of a whole output, as data URLs, in order. */
export function imagesOf(output: string): readonly string[] {
  try {
    const blocks = JSON.parse(output) as unknown
    if (!Array.isArray(blocks)) return []
    return blocks.flatMap((block) => {
      const source = block && typeof block === 'object' ? block.source : null
      return source && typeof source.data === 'string'
        ? [`data:${typeof source.media_type === 'string' ? source.media_type : 'image/png'};base64,${source.data}`]
        : []
    })
  } catch {
    return []
  }
}

/* The text of a whole output beside its images: its text blocks, or all of it
   when it is not blocks. */
export function textOf(output: string): string {
  try {
    const blocks = JSON.parse(output) as unknown
    if (!Array.isArray(blocks)) return output
    return blocks.flatMap((block) => (block && typeof block.text === 'string' ? [block.text] : [])).join('\n')
  } catch {
    return output
  }
}

