import type { PrefsPane } from './shape'

/*
 * The rows of General and of the panes it was split into, by the title each
 * row draws. Search reads it to find a row by its own name, and an address
 * from before the split (`general` and a row) to land where the row is now.
 * Apart from the sidebar's list so the shell can resolve an address without
 * loading the sidebar's icons.
 */
export const ROWS: Partial<Record<PrefsPane, readonly string[]>> = {
  general: [
    'Local by default',
    'Open at login',
    'Keep devpit in the tray when the window closes',
    'Shortcut',
    'Display',
    'Open in',
    'Automatic updates',
    'Updates',
    'Error reports',
  ],
  notifications: ['Island', 'Pause', 'Reminders', 'Focus mode'],
  sessions: ['Open the next card on its own', 'Hand off a full session', 'Ask before stopping a terminal'],
  voice: ['Voice messages'],
  remote: ['Remote', 'Allow remote MCP (read-only)', 'Calendar'],
}

/** The pane an address opens: General's old rows are found where they went. */
export function landing(pane: PrefsPane, row?: string | null): PrefsPane {
  const wanted = row?.trim().toLowerCase()
  if (pane !== 'general' || !wanted) return pane
  const holds = (id: PrefsPane): boolean => (ROWS[id] ?? []).some((title) => title.toLowerCase() === wanted)
  return (Object.keys(ROWS) as PrefsPane[]).find(holds) ?? pane
}
