import type { Strip, Tab } from './strip'

/* Which tabs a project had open, per project.
 *
 * The window opens on the empty state and restores what you left in this
 * project — a fixed starting tab is the app deciding what you were doing. */
const KEY = 'devpit.tabs'

type Saved = Record<string, { open: Tab[]; active: string | null }>

function all(): Saved {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '{}') as Saved
  } catch {
    return {}
  }
}

export const empty: Strip = { open: [], active: null }

export function remembered(projectId: string | null): Strip {
  if (!projectId) return empty
  const saved = all()[projectId]
  return saved ? { open: saved.open, active: saved.active } : empty
}

export function remember(projectId: string | null, strip: Strip): void {
  if (!projectId) return
  try {
    localStorage.setItem(
      KEY,
      JSON.stringify({ ...all(), [projectId]: { open: [...strip.open], active: strip.active } }),
    )
  } catch {
    /* A window that cannot write its view state still has to open. */
  }
}
