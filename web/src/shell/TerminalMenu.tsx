import type { Terminal } from '@xterm/xterm'
import { useEffect, useLayoutEffect, useRef } from 'react'
import { createPortal } from 'react-dom'

import { Clipboard, Close, Columns, Copy, Plus, Rows, SelectAll } from './GitIcons'
import { SHORTCUTS } from './shortcuts'
import { clipboardLabels, copySelection, pasteClipboard } from './terminalClipboard'
import { abandoned } from './typing'

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
  const menu = useRef<HTMLDivElement>(null)
  const keys = clipboardLabels()

  /* Opened near an edge, nudged back inside rather than drawn half off. */
  useLayoutEffect(() => {
    const el = menu.current
    if (!el) return
    const box = el.getBoundingClientRect()
    el.style.left = `${Math.min(at.x, window.innerWidth - box.width - 8)}px`
    el.style.top = `${Math.min(at.y, window.innerHeight - box.height - 8)}px`
  }, [at])

  useEffect(() => {
    const outside = (event: MouseEvent): void => {
      if (!menu.current?.contains(event.target as Node)) onClose()
    }
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) onClose()
    }
    document.addEventListener('mousedown', outside, true)
    document.addEventListener('keydown', key)
    window.addEventListener('blur', onClose)
    return () => {
      document.removeEventListener('mousedown', outside, true)
      document.removeEventListener('keydown', key)
      window.removeEventListener('blur', onClose)
    }
  }, [onClose])

  const run = (act: () => void | Promise<unknown>) => () => {
    onClose()
    void Promise.resolve(act()).finally(() => terminal.focus())
  }

  const item = (label: string, glyph: React.ReactNode, act: () => void | Promise<unknown>, key?: string, disabled = false): React.JSX.Element => (
    <button className="ctx__i" role="menuitem" disabled={disabled} onClick={run(act)}>
      <span className="ctx__g">{glyph}</span>
      {label}
      {key && <span className="ctx__k">{key}</span>}
    </button>
  )

  /* On the body: a pane slides in on a transform, and a fixed box inside a
     transformed one is placed against the pane, not the window. */
  return createPortal(
    <div className="ctx" ref={menu} role="menu" style={{ left: at.x, top: at.y }}>
      {item('Copy', <Copy />, () => copySelection(terminal), keys.copy, !terminal.hasSelection())}
      {item('Select All', <SelectAll />, () => terminal.selectAll())}
      {item('Paste', <Clipboard />, () => pasteClipboard(terminal, projectId).then(onFailed), keys.paste)}
      {onSplit && (
        <>
          <div className="ctx__rule" />
          {item('Split Right', <Columns />, () => onSplit('horizontal'), SHORTCUTS.splitRight)}
          {item('Split Down', <Rows />, () => onSplit('vertical'), SHORTCUTS.splitDown)}
        </>
      )}
      {onBlocks && (
        <>
          <div className="ctx__rule" />
          {item('Show as Blocks', <Rows />, onBlocks)}
        </>
      )}
      {(onClosePane || onSeparate) && <div className="ctx__rule" />}
      {onSeparate && item('Move to New Tab', <Plus />, onSeparate)}
      {onClosePane && item('Close Pane', <Close />, onClosePane)}
    </div>,
    document.body,
  )
}
