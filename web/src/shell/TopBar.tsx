import { useState } from 'react'

import mark from '../assets/brand/mark.png'
import { BranchPicker } from './BranchPicker'
import { HeadsDown } from './HeadsDown'
import { Notices } from './Notices'
import { ProjectPicker } from './ProjectPicker'
import { TabStrip } from './TabStrip'
import { useNotices } from './useNotices'
import { useShell } from './useShell'
import { useTree } from './useTree'
import { close, minimize, toggleMaximize } from './window'

/*
 * The window is one project, so the project sits above the columns rather than
 * inside one of them. The lead block is exactly as wide as the sidebar under
 * it, so the first tab starts on the line the content starts on.
 *
 * The two panel toggles sit together on the right, next to the window
 * controls: they are one job — showing and hiding the columns — and split
 * apart they read as two unrelated buttons.
 *
 * The bar carries `data-tauri-drag-region`, so it is the window's handle.
 * Everything interactive in it is a child without the attribute, which is why
 * the buttons still take their own clicks.
 */
export function TopBar({ onAddProject }: { onAddProject: () => void }): React.JSX.Element {
  const { side, files, toggleSide, toggleFiles, project, show, openCard } = useShell()
  /* One reader for the whole top bar: the panel and the pill draw from the
     same list, so a count on one cannot disagree with the other. */
  const bell = useNotices()
  const [bellOpen, setBellOpen] = useState(false)
  const { totals } = useTree(project?.id ?? null)
  const here = project?.worktrees.find((tree) => tree.current) ?? project?.worktrees[0]

  return (
    <header className="top" data-tauri-drag-region>
      <div className="top__lead">
        <span className="logo">
          <img className="mark" alt="devpit" src={mark} />
        </span>
        <ProjectPicker onAdd={onAddProject} />
      </div>

      <TabStrip />

      <span className="drag" data-tauri-drag-region />

      <HeadsDown
        projectId={project?.id ?? null}
        waiting={bell.waiting.length}
        onPeek={() => setBellOpen(true)}
        onOpenCard={(cardId) => {
          show('board')
          openCard(cardId)
        }}
      />
      {here && <BranchPicker branch={here.branch} ahead={here.ahead} />}
      <span className="netstat" hidden={totals.added + totals.removed === 0}>
        <span className="add">+{totals.added}</span>
        <span className="del">&minus;{totals.removed}</span>
      </span>

      {/* Beside the panel toggles rather than in the sidebar: what it has to
          say is about the whole window, and half of it arrives while the
          sidebar is hidden. */}
      <Notices
        bell={bell}
        open={bellOpen}
        setOpen={setBellOpen}
        onOpenCard={(cardId) => {
          show('board')
          openCard(cardId)
        }}
      />

      <button
        className="sq26 tip"
        data-tip={`${side ? 'Hide' : 'Show'} the sidebar`}
        aria-label={`${side ? 'Hide' : 'Show'} the sidebar`}
        onClick={toggleSide}
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="3" width="18" height="18" rx="2" />
          <path d="M9 3v18" />
        </svg>
      </button>
      <button
        className="sq26 tip"
        data-tip={`${files ? 'Hide' : 'Show'} the files panel`}
        aria-label={`${files ? 'Hide' : 'Show'} the files panel`}
        onClick={toggleFiles}
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="3" width="18" height="18" rx="2" />
          <path d="M15 3v18" />
        </svg>
      </button>

      {/* A frameless window still has to be minimised, maximised and closed;
          these are ours to draw because the system's are not there. */}
      <span className="wctl">
        <button className="wbtn" aria-label="Minimize" onClick={minimize}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round">
            <path d="M5 12h14" />
          </svg>
        </button>
        <button className="wbtn" aria-label="Maximize" onClick={toggleMaximize}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7">
            <rect x="5" y="5" width="14" height="14" rx="1.5" />
          </svg>
        </button>
        <button className="wbtn wbtn--x" aria-label="Close" onClick={close}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </span>
    </header>
  )
}
