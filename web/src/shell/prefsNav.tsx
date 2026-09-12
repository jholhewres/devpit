import type { PrefsPane } from './shape'

/*
 * The settings sidebar, as a list rather than as markup.
 *
 * Nine buttons in a flat column meant the only way to find a setting was to
 * have already known where it was. Grouped, the column answers a question a
 * newcomer can actually ask — "this is about my projects" — before it asks
 * them to know the name of the pane.
 *
 * A list because two things read it: the column that draws it, and the search
 * field that filters it. Written as markup, the second one would have had to
 * re-state the first.
 */

export interface NavItem {
  readonly id: PrefsPane
  readonly label: string
  /** What else this pane is about, for someone searching the word they know
   *  rather than the word on the button. */
  readonly about: string
  readonly icon: React.JSX.Element
}

export interface NavGroup {
  readonly id: string
  readonly title: string
  readonly items: readonly NavItem[]
}

const stroke = {
  width: 15,
  height: 15,
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.9,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
} as const

export const PREFS_NAV: readonly NavGroup[] = [
  {
    id: 'application',
    title: 'Application',
    items: [
      {
        id: 'general',
        label: 'General',
        about: 'updates telemetry privacy anonymous data local',
        icon: (
          <svg {...stroke}><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-2.82 1.18V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 7.26 19.4l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 3.09 13H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 7.26l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 10 3.09V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 2.74 1.51l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 20.91 11H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z" /></svg>
        ),
      },
      {
        id: 'appearance',
        label: 'Appearance',
        about: 'theme dark light system colours terminal',
        icon: (
          <svg {...stroke}><circle cx="12" cy="12" r="9" /><path d="M12 3v18" /></svg>
        ),
      },
    ],
  },
  {
    id: 'workspace',
    title: 'Workspace',
    items: [
      {
        id: 'projects',
        label: 'Projects',
        about: 'repositories folders clone remove rename remote origin',
        icon: (
          <svg {...stroke}><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" /></svg>
        ),
      },
      {
        id: 'worktrees',
        label: 'Worktrees',
        about: 'checkouts branches disk cards',
        icon: (
          <svg {...stroke}><line x1="6" y1="3" x2="6" y2="15" /><circle cx="18" cy="6" r="3" /><circle cx="6" cy="18" r="3" /><path d="M18 9a9 9 0 0 1-9 9" /></svg>
        ),
      },
      {
        id: 'storage',
        label: 'Storage',
        about: 'disk transcripts workspace directory size',
        icon: (
          <svg {...stroke}><ellipse cx="12" cy="6" rx="8" ry="3" /><path d="M4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6" /><path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" /></svg>
        ),
      },
    ],
  },
  {
    id: 'capabilities',
    title: 'AI capabilities',
    items: [
      {
        id: 'providers',
        label: 'Providers',
        about: 'agents claude codex gemini models cli',
        icon: (
          <svg {...stroke}><rect x="3" y="3" width="18" height="18" rx="3" /><path d="m8 10 3 3-3 3M14 16h3" /></svg>
        ),
      },
      {
        id: 'skills',
        label: 'Skills',
        about: 'capabilities prompts omc installed',
        icon: (
          <svg {...stroke}><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg>
        ),
      },
    ],
  },
  {
    id: 'account',
    title: 'Account',
    items: [
      {
        id: 'account',
        label: 'Account',
        about: 'sign in out sync email profile',
        icon: (
          <svg {...stroke}><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg>
        ),
      },
      {
        id: 'usage',
        label: 'Usage',
        about: 'spend cost tokens budget limits',
        icon: (
          <svg {...stroke}><path d="M4 20V10M10 20V4M16 20v-7M22 20H2" /></svg>
        ),
      },
    ],
  },
]

/*
 * The groups that still have something in them, for a given query.
 *
 * Empty query is the whole list untouched rather than a filter that happens to
 * match everything: a heading that disappears while you delete the last letter
 * of your search is the column moving under the pointer.
 */
export function matching(query: string, nav: readonly NavGroup[] = PREFS_NAV): readonly NavGroup[] {
  const wanted = query.trim().toLowerCase()
  if (!wanted) return nav

  return nav
    .map((group) => ({
      ...group,
      items: group.items.filter(
        (item) =>
          item.label.toLowerCase().includes(wanted) ||
          item.about.includes(wanted) ||
          group.title.toLowerCase().includes(wanted),
      ),
    }))
    .filter((group) => group.items.length > 0)
}
