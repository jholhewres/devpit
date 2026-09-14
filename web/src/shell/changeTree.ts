import type { Change } from '../gen/bindings'

/*
 * The changed files, as the folders they are in.
 *
 * A flat list is fine for three files and unreadable for forty: `mod.rs` four
 * times over tells you nothing, and the full path on every row is a column of
 * repeated prefixes in a panel narrow enough to be a sidebar.
 *
 * Runs of folders with one child collapse into one row — `apps/desktop/src`
 * rather than three rows each containing only the next. Depth is not
 * information; it is the price of finding out where something is.
 */

export interface Folder {
  readonly kind: 'folder'
  /** The whole path, which is what an open/closed set is keyed by. */
  readonly path: string
  /** What to draw: the collapsed run, not just the last segment. */
  readonly name: string
  readonly files: number
  readonly children: readonly Node[]
}

export interface File {
  readonly kind: 'file'
  readonly change: Change
  /** The last segment. The folder above it says the rest. */
  readonly name: string
}

export type Node = Folder | File

interface Building {
  folders: Map<string, Building>
  files: Change[]
}

const empty = (): Building => ({ folders: new Map(), files: [] })

/** The changes, arranged as folders. */
export function foldered(changes: readonly Change[]): readonly Node[] {
  const root = empty()
  for (const change of changes) {
    const parts = change.path.split('/')
    parts.pop()
    let here = root
    for (const part of parts) {
      let next = here.folders.get(part)
      if (!next) {
        next = empty()
        here.folders.set(part, next)
      }
      here = next
    }
    here.files.push(change)
  }
  return laid(root, '')
}

function laid(here: Building, prefix: string): readonly Node[] {
  const folders: Node[] = []
  for (const [name, child] of here.folders) {
    folders.push(folded(name, child, prefix ? `${prefix}/${name}` : name))
  }
  /* Folders first, then files, each by name — the order a file tree has
     everywhere, so nobody has to learn this one. */
  folders.sort((one, two) => one.name.localeCompare(two.name))
  const files: Node[] = here.files
    .map((change) => ({
      kind: 'file' as const,
      change,
      name: change.path.split('/').pop() ?? change.path,
    }))
    .sort((one, two) => one.name.localeCompare(two.name))
  return [...folders, ...files]
}

/** One folder, with any single-child run below it folded into its name. */
function folded(name: string, here: Building, path: string): Folder {
  let said = name
  let at = here
  let where = path
  // Only a run of folders with nothing else in them. A folder holding one
  // file and one folder is two things and stays two rows.
  while (at.files.length === 0 && at.folders.size === 1) {
    const [only, child] = [...at.folders][0]!
    said = `${said}/${only}`
    where = `${where}/${only}`
    at = child
  }
  return {
    kind: 'folder',
    path: where,
    name: said,
    files: counted(at),
    children: laid(at, where),
  }
}

function counted(here: Building): number {
  let total = here.files.length
  for (const child of here.folders.values()) total += counted(child)
  return total
}

/** Every folder path in the tree, for opening or closing all of them. */
export function folders(nodes: readonly Node[]): string[] {
  return nodes.flatMap((node) =>
    node.kind === 'folder' ? [node.path, ...folders(node.children)] : [],
  )
}

/** Every file under these nodes, for staging a folder in one click. */
export function paths(nodes: readonly Node[]): string[] {
  return nodes.flatMap((node) =>
    node.kind === 'folder' ? paths(node.children) : [node.change.path],
  )
}
