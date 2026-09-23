import type { Terminal } from '@xterm/xterm'

import { Clipboard, Close, Columns, Copy, Plus, Rows, SelectAll } from './GitIcons'
import { Menu, MenuItem, MenuRule } from './Menu'
import { SHORTCUTS } from './shortcuts'
import { clipboardLabels, copySelection, pasteClipboard } from './terminalClipboard'

/*
 * A terminal's right-click, in Orca's order: what to do with text first, then
 * what to do with the pane.
 *
 * The window's own context menu leaves text fields to the system, and a
 * terminal's input is a hidden textarea — so right-click on a terminal used
 * to offer the webview's menu, which cannot reach xterm's selection.
 *
 * No Clear. Clearing is the program's business: xterm wiping its buffer under
 * a full-screen program like Claude Code leaves that program drawing into
 * rows it believes are still there, and the pane comes apart.
 */

export function TerminalMenu({
  at,
  terminal,
  projectId,
  onClose,
  onFailed,
  onSplit,
  onClosePane,
  onBlocks,
  onSeparate,
}: {
  at: { readonly x: number; readonly y: number }
  terminal: Terminal
  projectId: string
  onClose: () => void
  onFailed: (why: string | null) => void
  onSplit?: (direction: 'horizontal' | 'vertical') => void
  onClosePane?: () => void
  onBlocks?: () => void
  /** Absent for the only pane of a tab, which already has a tab of its own. */
  onSeparate?: () => void
}): React.JSX.Element {
  const keys = clipboardLabels()

  const run = (act: () => void | Promise<unknown>) => () => {
    onClose()
    void Promise.resolve(act()).finally(() => terminal.focus())
  }

  return (
    <Menu at={at} label="Terminal actions" onClose={onClose}>
      <MenuItem label="Copy" glyph={<Copy />} keys={keys.copy} disabled={!terminal.hasSelection()} onPick={run(() => copySelection(terminal))} />
      <MenuItem label="Select all" glyph={<SelectAll />} onPick={run(() => terminal.selectAll())} />
      <MenuItem label="Paste" glyph={<Clipboard />} keys={keys.paste} onPick={run(() => pasteClipboard(terminal, projectId).then(onFailed))} />
      {onSplit && (
        <>
          <MenuRule />
          <MenuItem label="Split right" glyph={<Columns />} keys={SHORTCUTS.splitRight} onPick={run(() => onSplit('horizontal'))} />
          <MenuItem label="Split down" glyph={<Rows />} keys={SHORTCUTS.splitDown} onPick={run(() => onSplit('vertical'))} />
        </>
      )}
      {onBlocks && (
        <>
          <MenuRule />
          <MenuItem label="Show as blocks" glyph={<Rows />} onPick={run(onBlocks)} />
        </>
      )}
      {(onClosePane || onSeparate) && <MenuRule />}
      {onSeparate && <MenuItem label="Move to new tab" glyph={<Plus />} onPick={run(onSeparate)} />}
      {onClosePane && <MenuItem label="Close pane" glyph={<Close />} bad onPick={run(onClosePane)} />}
    </Menu>
  )
}
