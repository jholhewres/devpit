/* What the Explorer panel was showing, per project.
 *
 * Same shape as `tabs.ts`: the panel never unmounts, so within one session
 * this would survive on its own, but a project switch has no reason to
 * carry Project A's search query onto Project B, and a real restart has
 * nothing to restore from without this. */
const KEY = 'devpit.explorer'

export type View = 'tree' | 'changes' | 'history'
export type Mode = 'names' | 'contents'

export interface ExplorerState {
  readonly view: View
  readonly mode: Mode
  readonly query: string
}

type Saved = Record<string, ExplorerState>

function all(): Saved {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '{}') as Saved
  } catch {
    return {}
  }
}

export const empty: ExplorerState = { view: 'tree', mode: 'names', query: '' }

export function remembered(projectId: string | null): ExplorerState {
  if (!projectId) return empty
  return all()[projectId] ?? empty
}

export function remember(projectId: string | null, state: ExplorerState): void {
  if (!projectId) return
  try {
    localStorage.setItem(KEY, JSON.stringify({ ...all(), [projectId]: state }))
  } catch {
    /* A window that cannot write its view state still has to open. */
  }
}
