/*
 * The lines run in a project's terminals, newest first, for the input's
 * suggestions and its history.
 *
 * Per project and per window, like the rest of what the terminal remembers
 * about you. Kept short and without repeats: history is for finding the line
 * you ran yesterday, not an audit of every time you typed `ls`.
 */

const KEY = (projectId: string): string => `devpit.terminal.history.${projectId}`
export const MOST_KEPT = 500

export function recalled(projectId: string): readonly string[] {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(KEY(projectId)) ?? '[]')
    return Array.isArray(raw) ? raw.filter((one): one is string => typeof one === 'string') : []
  } catch {
    return []
  }
}

/** The history with `line` at its front, and not again further back. */
export function withLine(history: readonly string[], line: string): readonly string[] {
  const kept = line.trim()
  if (!kept) return history
  return [kept, ...history.filter((one) => one !== kept)].slice(0, MOST_KEPT)
}

export function remember(projectId: string, line: string): readonly string[] {
  const next = withLine(recalled(projectId), line)
  try {
    localStorage.setItem(KEY(projectId), JSON.stringify(next))
  } catch {
    /* Still offered for as long as the window is open. */
  }
  return next
}

/** The rest of the newest line that starts with what was typed: the ghost text. */
export function suggestion(history: readonly string[], typed: string): string | null {
  if (!typed || typed.includes('\n')) return null
  const found = history.find((one) => one.startsWith(typed) && one !== typed)
  return found ? found.slice(typed.length) : null
}

/** The lines a search matches, newest first: every word typed, in any order. */
export function searched(history: readonly string[], query: string, most = 50): readonly string[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean)
  return history.filter((one) => words.every((word) => one.toLowerCase().includes(word))).slice(0, most)
}
