import { createContext } from 'react'

/*
 * A tool's output the window was sent cut: long text as its head and tail,
 * an image by its size. The transcript keeps it whole; opening the row asks
 * for it by the call it answers.
 */

/* The line the backend writes where it left text out (`crates/rpc/src/cut.rs`). */
const CUT_MARK = '… [cut: '

export function isCut(output: string): boolean {
  return output.includes(CUT_MARK) || output.includes('"cut":true')
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
