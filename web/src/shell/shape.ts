import type { PaneRunning, Project, Theme as StoredTheme } from '../gen/bindings'
import type { PaneName } from './paneList'
import type { Tab } from './strip'
import type { Who } from './account'
import type { Membership } from './useAccount'
import type { Doing } from './useAgents'
import type { Closing } from './useClosing'
import type { Renaming } from './useTabs'

/*
 * What the window has, in one shape.
 *
 * Apart from the provider that fills it because the list is long and the
 * filling is short: reading what a surface can reach should not mean scrolling
 * past the wiring that gets it there.
 */

export type Theme = StoredTheme
export type PrefsPane =
  | 'account'
  | 'projects'
  | 'general'
  | 'appearance'
  | 'providers'
  | 'skills'
  | 'storage'
  | 'worktrees'
  | 'usage'

/*
 * Three facts about the panes, because they came apart the moment tabs
 * arrived:
 *
 *   open      the strip's order — first opened is leftmost, and a new one
 *             lands on the right. Clicking an existing tab must not move it,
 *             or the strip reshuffles under the pointer.
 *   active    the pane you are looking at.
 *   previous  what a split would pair with — recency, which is a different
 *             order from the strip and the reason these are two lists.
 */
export interface Shell {
  readonly open: readonly Tab[]
  readonly active: Tab | null
  /** Opens a new one of a kind you can have several of; focuses the rest. */
  show: (kind: PaneName, tab?: Partial<Tab>) => void
  close: (id: string) => void
  focus: (id: string) => void
  move: (id: string, to: number) => void
  rename: (id: string, title: string) => void
  attach: (id: string, panes: readonly string[]) => void
  launched: (id: string) => void
  /** What each of the project's panes has in front of it.

      Polled once for the window rather than once per surface: the strip, the
      sidebar and the close prompt all ask the same question, and three timers
      would be three answers that disagree for two seconds at a time. */
  readonly running: readonly PaneRunning[]
  /** What each pane's agent last said it was doing.

      Apart from `running` because the two are different questions with
      different answers: the process table says which agent is open, and only
      the agent says whether it is working or waiting for you. */
  readonly doing: Doing
  readonly renaming: Renaming | null
  setRenaming: (renaming: Renaming | null) => void

  readonly side: boolean
  readonly files: boolean
  toggleSide: () => void
  toggleFiles: () => void

  readonly theme: Theme
  setTheme: (theme: Theme) => void

  readonly project: Project | null
  readonly projects: readonly Project[]
  readonly projectsError: string | null
  setProject: (id: string) => void
  forgetProject: (id: string, wipe?: boolean) => void
  renameProject: (id: string, name: string) => Promise<string | null>
  reloadProjects: () => void

  readonly signedIn: boolean
  readonly account: Who
  /** The whole sign-in machine, for the sheet that drives it. The two fields
   *  above are what every other pane needs, and they stay. */
  readonly membership: Membership
  signIn: () => void
  signOut: () => void

  readonly prefs: PrefsPane | null
  openPrefs: (pane?: PrefsPane) => void
  closePrefs: () => void

  /** The card a notification asked for, and the board's job to clear.

      On the shell because two surfaces have to agree about it and neither
      contains the other: the bell is in the top bar, and the card opens over
      the board. Cleared by whoever opened it, so a second click on the same
      notification opens it again. */
  readonly wantedCard: string | null
  openCard: (cardId: string | null) => void

  /** The one field over the window. Here rather than in the window's own
      state because three things open it — the sidebar's Search, ⌘K, and the
      plus at the end of the strip — and two of them are not the window. */
  readonly palette: boolean
  openPalette: () => void
  closePalette: () => void

  /** The close that stopped to ask, because something is running in it.

      Closing a terminal kills the window behind it, and an agent halfway
      through a task is not something to lose to a stray click on a cross. */
  readonly closing: Closing | null
  confirmClose: (dontAskAgain: boolean) => void
  cancelClose: () => void
}
