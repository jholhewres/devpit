import { Drawing } from '../plugins/excalidraw/Drawing'
import { NotePane } from '../plugins/notes/NotePane'
import { NotesList } from '../plugins/notes/NotesList'
import { DrawingList } from '../plugins/excalidraw/DrawingList'
import { BoardPane } from './BoardPane'
import { ChatPane } from './ChatPane'
import { DiffPane } from './DiffPane'
import { FilePane } from './FilePane'
import { FilesPane } from './FilesPane'
import { McpPane } from './McpPane'
import type { PaneName } from './paneList'
import { PluginsPane } from './PluginsPane'
import { SkillsPane } from './SkillsPane'
import type { Tab } from './strip'
import { TerminalPane } from './TerminalPane'
import { WorkspacePane } from './WorkspacePane'

/* One row per pane kind: what mounts it, and whether it is drawn once or
 * once per open tab. A new kind is a row here plus a row in `PANES`. */

interface Single {
  readonly name: PaneName
  readonly many: false
  readonly className: string
  render(close: (id: string) => void): React.JSX.Element
}

interface Many {
  readonly name: PaneName
  readonly many: true
  readonly className: string
  render(tab: Tab): React.JSX.Element
}

export type PaneMount = Single | Many

const closeIcon = (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
    <path d="M18 6 6 18M6 6l12 12" />
  </svg>
)

export const PANE_MOUNTS: readonly PaneMount[] = [
  {
    name: 'board',
    many: false,
    className: 'pane',
    render: (close) => (
      <>
        <div className="pane__bar">
          <span className="pane__ico">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg>
          </span>
          <span className="pane__t"><b>Board</b></span>
          <span className="drag"></span>
          <button className="sq26" onClick={() => close('board')} aria-label="Close board">{closeIcon}</button>
        </div>
        <BoardPane />
      </>
    ),
  },
  {
    /* The bar lives here and not in `SkillsPane`, which is also mounted
       inside Settings — where this chrome would be a second title bar under
       the first. */
    name: 'skills',
    many: false,
    className: 'pane',
    render: (close) => (
      <>
        <div className="pane__bar">
          <span className="pane__ico">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" /><rect x="7" y="12" width="10" height="9" rx="2" /></svg>
          </span>
          <span className="pane__t"><b>Skills</b></span>
          <span className="drag"></span>
          <button className="sq26" onClick={() => close('skills')} aria-label="Close Skills">{closeIcon}</button>
        </div>
        <SkillsPane />
      </>
    ),
  },
  { name: 'mcps', many: false, className: 'pane', render: () => <McpPane /> },
  { name: 'plugins', many: false, className: 'pane', render: () => <PluginsPane /> },
  { name: 'files', many: false, className: 'pane', render: () => <FilesPane /> },
  { name: 'workspace', many: false, className: 'pane', render: () => <WorkspacePane /> },
  { name: 'chat', many: true, className: 'pane', render: (tab) => <ChatPane tab={tab} /> },
  { name: 'diff', many: true, className: 'pane pane--file', render: (tab) => <DiffPane tab={tab} /> },
  { name: 'file', many: true, className: 'pane pane--file', render: (tab) => <FilePane tab={tab} /> },
  { name: 'term', many: true, className: 'pane pane--term', render: (tab) => <TerminalPane tab={tab} /> },
  /* A note tab with no file is the list of notes. */
  {
    name: 'note',
    many: true,
    className: 'pane',
    render: (tab) => (tab.path ? <NotePane tab={tab} name={tab.path} /> : <NotesList tab={tab} />),
  },
  /* A drawing tab with no file is the list of drawings. */
  {
    name: 'drawing',
    many: true,
    className: 'pane',
    render: (tab) => (tab.path ? <Drawing tab={tab} name={tab.path} /> : <DrawingList tab={tab} />),
  },
]
