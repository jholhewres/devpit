import type { FileNode, GitStatus } from '../gen/bindings'

/* Folders first, then files, each alphabetical — the order every tree uses,
   and the reason a refresh does not reshuffle what you were looking at. */
export function ordered(nodes: readonly FileNode[]): readonly FileNode[] {
  return [...nodes].sort((a, b) => {
    const folder = (node: FileNode): number => (node.children === null ? 1 : 0)
    return folder(a) - folder(b) || a.name.localeCompare(b.name)
  })
}

/* The letter git puts beside a path. Untracked is `?` on the wire and `A`
   to a reader — nobody scans a column of question marks. */
export function mark(status: GitStatus): string | null {
  switch (status) {
    case 'modified':
      return 'M'
    case 'added':
    case 'untracked':
      return 'A'
    case 'deleted':
      return 'D'
    default:
      return null
  }
}

/* What a version bump means for a folder's cached children: never fetched is
   nothing to do, open is worth a fresh fetch, closed just drops the cache so
   the next open does not show what a reload already knows is stale. */
export type ChildRefresh = 'skip' | 'now' | 'drop'

export function refreshChildren(hasChildren: boolean, open: boolean): ChildRefresh {
  if (!hasChildren) return 'skip'
  return open ? 'now' : 'drop'
}

/* Which paths a filter keeps: a file that matches, and every folder on the
   way to it — a match nobody can reach is a match nobody sees. */
export function matching(nodes: readonly FileNode[], query: string): Set<string> {
  const keep = new Set<string>()
  const wanted = query.trim().toLowerCase()
  if (!wanted) return keep

  const walk = (node: FileNode, parents: string[]): void => {
    const hit = node.name.toLowerCase().includes(wanted)
    if (hit) {
      keep.add(node.path)
      parents.forEach((parent) => keep.add(parent))
    }
    node.children?.forEach((child) => walk(child, [...parents, node.path]))
  }
  nodes.forEach((node) => walk(node, []))
  return keep
}
