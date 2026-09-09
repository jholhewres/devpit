import mark from '../assets/brand/mark.png'
import { BRANCH } from '../mock/data'
import { ProjectPicker } from './ProjectPicker'
import { TabStrip } from './TabStrip'
import { useShell } from './useShell'
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
  const { side, files, toggleSide, toggleFiles } = useShell()

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

      <button className="branch" title="Switch branch">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
          <line x1="6" y1="3" x2="6" y2="15" />
          <circle cx="18" cy="6" r="3" />
          <circle cx="6" cy="18" r="3" />
          <path d="M18 9a9 9 0 0 1-9 9" />
        </svg>
        <span className="branch__n">{BRANCH.name}</span>
        <span className="branch__ahead">&uarr;{BRANCH.ahead}</span>
      </button>
      <span className="netstat">
        <span className="add">+{BRANCH.added}</span>
        <span className="del">&minus;{BRANCH.deleted}</span>
      </span>

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
