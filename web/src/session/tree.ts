import type { LayoutNode } from '../gen/bindings'

export function leafIds(node: LayoutNode): string[] {
  if (node.type === 'leaf') return [node.id]
  return [...leafIds(node.first), ...leafIds(node.second)]
}
