import type { Project } from '../gen/bindings'
import { inTauri } from './window'

/*
 * The shell, opened in a plain browser.
 *
 * Outside Tauri there is no backend: every command fails, the project list
 * comes back empty, and the onboarding screen — whose whole job is to be the
 * empty state — covers the window. So the layout, the menus and the chat
 * chrome cannot be looked at without building the desktop app.
 *
 * `?preview` hands the shell one project that does not exist, which is enough
 * for everything that does not touch disk. It is deliberately a query string
 * rather than a setting: it lasts exactly as long as the tab, cannot be left
 * on by accident, and is inert inside Tauri — where a stand-in project would
 * be a lie about what the app has open.
 */

export const previewing = (): boolean =>
  !inTauri() &&
  typeof window !== 'undefined' &&
  new URLSearchParams(window.location.search).has('preview')

export const STANDIN: Project = {
  id: 'preview',
  name: 'preview',
  rootPath: '/preview',
  group: null,
  accent: '#e2795b',
  worktrees: [],
  unreadable: null,
    orchestrator: null,
  origin: null,
  lastOpenedAt: null,
  icon: null,
  color: null,
}
