import { writeText } from '@tauri-apps/plugin-clipboard-manager'

import type { Change } from '../gen/bindings'
import { ask, commands } from './live'
import { Menu, MenuItem, MenuRule } from './Menu'
import { went } from './problems'
import { useShellPick } from './shellStore'

/*
 * Right-click on a changed file in the git panel: what can be done to it.
 *
 * The same actions the row's hover buttons offer, plus the ones a person
 * reaches for next — its diff, the file itself, its path, where it is on
 * disk. Discard is last and red, and still asks first (`onDiscard`).
 */
export function ChangeMenu({
  change,
  at,
  staged,
  busy,
  onOpen,
  onStage,
  onDiscard,
  onClose,
}: {
  change: Change
  at: { readonly x: number; readonly y: number }
  staged: boolean
  /** Another git command on the index is still going. */
  busy: boolean
  onOpen: (path: string) => void
  onStage: (paths: string[]) => void
  onDiscard: (change: Change) => void
  onClose: () => void
}): React.JSX.Element {
  const { root, show } = useShellPick((shell) => ({ root: shell.project?.rootPath ?? null, show: shell.show }))
  const then = (act: () => void) => (): void => {
    onClose()
    act()
  }
  const path = change.path
  return (
    <Menu at={at} label="Change actions" onClose={onClose}>
      <MenuItem label="Open diff" onPick={then(() => onOpen(path))} />
      {change.status !== 'deleted' && (
        <MenuItem label="Open file" onPick={then(() => show('file', { id: `file:${path}`, path, title: path.split('/').pop() }))} />
      )}
      <MenuRule />
      <MenuItem label={staged ? 'Unstage' : 'Stage'} disabled={busy} onPick={then(() => onStage([path]))} />
      <MenuItem label="Copy path" onPick={then(() => void writeText(path).catch(() => went({ error: 'the path could not be copied' })))} />
      {root && change.status !== 'deleted' && (
        <MenuItem label="Reveal in the file manager" onPick={then(() => void ask(() => commands.pathReveal(`${root}/${path}`)).then(went))} />
      )}
      <MenuRule />
      <MenuItem label={change.status === 'untracked' ? 'Delete…' : 'Discard changes…'} bad disabled={busy} onPick={then(() => onDiscard(change))} />
    </Menu>
  )
}
