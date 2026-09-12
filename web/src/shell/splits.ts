import type { LayoutNode } from '../gen/bindings'

/*
 * Where a boundary sits, and what a drag does to it.
 *
 * The tree comes from the backend and is persisted; this is only the geometry
 * — which is exactly the part that is easy to get subtly wrong and impossible
 * to notice from reading.
 */

/** Every leaf in a tree, in the order it is drawn. */
export function leaves(node: LayoutNode): readonly string[] {
  return node.type === 'leaf' ? [node.id] : [...leaves(node.first), ...leaves(node.second)]
}

/** How many panes a tab is showing. */
export const paneCount = (node: LayoutNode): number => leaves(node).length

/*
 * A boundary never leaves either side too small to use.
 *
 * Not a matter of taste: a pane at two per cent is a pane whose terminal has
 * one column, and the shell inside it starts wrapping every character. The
 * floor is in fractions rather than pixels because the tree stores a ratio,
 * and a pixel floor would mean something different on every window size.
 */
export const LEAST = 0.08

export const boundedRatio = (ratio: number): number =>
  Math.min(1 - LEAST, Math.max(LEAST, ratio))

/**
 * The ratio a drag lands on.
 *
 * `along` is how far the pointer is down or across the split's own box, and
 * `size` is that box's length. A drag on a box with no size yet — the first
 * frame after a split, before layout — keeps what it had rather than dividing
 * by zero into a NaN the tree would then be persisted with.
 */
export function draggedTo(along: number, size: number, was: number): number {
  if (!(size > 0)) return was
  return boundedRatio(along / size)
}

/** The percentages a split's two children take, as CSS reads them. */
export function sides(ratio: number): { first: string; second: string } {
  const bounded = boundedRatio(ratio)
  return {
    first: `${(bounded * 100).toFixed(4)}%`,
    second: `${((1 - bounded) * 100).toFixed(4)}%`,
  }
}

/** The node with this id, or nothing. Used to read a boundary back after a
 *  drag without walking the tree twice at the call site. */
export function nodeAt(node: LayoutNode, id: string): LayoutNode | null {
  if (node.type === 'leaf') return node.id === id ? node : null
  if (node.id === id) return node
  return nodeAt(node.first, id) ?? nodeAt(node.second, id)
}
