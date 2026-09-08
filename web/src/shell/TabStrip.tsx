import type { Worktree } from '../gen/bindings'
import { PanelGlyph, PlusGlyph, SidebarGlyph } from './glyphs'

/**
 * The strip above the stage, which is also the titlebar.
 *
 * Terminals are tabs; a surface is not. The plus splits the focused leaf to
 * the right — Orca's horizontal split — and the session layer owns the process.
 */
export function TabStrip({
  worktree,
  paneCount,
  focusedId,
  sidebarOpen,
  panelOpen,
  canSplit,
  onToggleSidebar,
  onTogglePanel,
  onSplit
}: {
  worktree: Worktree | null
  paneCount: number
  focusedId: string | null
  sidebarOpen: boolean
  panelOpen: boolean
  canSplit: boolean
  onToggleSidebar: () => void
  onTogglePanel: () => void
  onSplit: () => void
}): React.JSX.Element {
  const label = focusedId
    ? `${worktree?.branch ?? 'shell'} · ${paneCount} pane${paneCount === 1 ? '' : 's'}`
    : worktree
      ? `${worktree.branch} — no terminal`
      : 'no checkout'

  return (
    <div className="tabs" data-tauri-drag-region>
      <button
        type="button"
        className="icon-button"
        data-active={sidebarOpen}
        title={sidebarOpen ? 'Hide sidebar' : 'Show sidebar'}
        onClick={onToggleSidebar}
      >
        <SidebarGlyph />
      </button>

      <span className="tabs__idle" data-tauri-drag-region>
        {label}
      </span>

      <button
        type="button"
        className="icon-button"
        title="Split right"
        disabled={!canSplit}
        onClick={onSplit}
      >
        <PlusGlyph size={14} />
      </button>

      <span className="tabs__spacer" data-tauri-drag-region />

      <div className="tabs__actions">
        <button
          type="button"
          className="icon-button"
          data-active={panelOpen}
          title={panelOpen ? 'Hide panel' : 'Show panel'}
          onClick={onTogglePanel}
        >
          <PanelGlyph />
        </button>
      </div>
    </div>
  )
}
