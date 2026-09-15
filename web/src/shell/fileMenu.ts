/*
 * The file row's context menu, as data.
 *
 * Apart from the component so a test can call `wired` on it. What this
 * replaces was four entries that closed the menu and did nothing, which the
 * `a_control_either_works_or_goes` guard cannot catch: it asks whether a
 * handler exists, not whether it does anything. An entry here either draws a
 * rule or names an action, and `wired` is what fails when one stops doing so.
 *
 * "Open beside" is gone rather than disabled. Nothing in the shell opens a
 * second pane yet, and AGENTS.md is explicit that a control which cannot be
 * wired leaves the screen — the markup is in git for when it becomes real.
 */

export type ActionId =
  | 'open'
  | 'copyPath'
  | 'reveal'
  | 'newFile'
  | 'newFolder'
  | 'rename'
  | 'delete'

export interface Entry {
  readonly label?: string
  readonly key?: string
  readonly rule?: true
  /** Drawn as destructive. Only what cannot be undone from here earns it. */
  readonly bad?: true
  readonly act?: ActionId
}

export const FILE_MENU: readonly Entry[] = [
  { label: 'Open', key: '↵', act: 'open' },
  { rule: true },
  { label: 'New file…', act: 'newFile' },
  { label: 'New folder…', act: 'newFolder' },
  { rule: true },
  { label: 'Copy path', act: 'copyPath' },
  { label: 'Reveal in the finder', act: 'reveal' },
  { rule: true },
  { label: 'Rename…', key: 'F2', act: 'rename' },
  { label: 'Delete', act: 'delete', bad: true },
]

/** Whether every entry either draws a rule or does something. */
export const wired = (menu: readonly (Entry & { readonly run?: unknown })[]): boolean =>
  menu.every((entry) => entry.rule === true || entry.act !== undefined || entry.run !== undefined)
