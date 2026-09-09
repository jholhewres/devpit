import mark from '../assets/brand/mark.png'
import { BoardPane } from './BoardPane'
import { ChatPane } from './ChatPane'
import { SkillsPane } from './SkillsPane'
import { DiffPane } from './DiffPane'
import { FilePane } from './FilePane'
import { FilesPane } from './FilesPane'
import { McpPane } from './McpPane'
import { TerminalPane } from './TerminalPane'
import { WorkspacePane } from './WorkspacePane'
import { useShell } from './useShell'

/*
 * One pane visible at a time, in the order the strip gives them.
 *
 * Diagram, Excalidraw and Capabilities were removed rather than wired: there
 * was nothing behind them — no registry, no installer, no runtime — and a tab
 * that opens a picture of a feature is the kind of lie this milestone exists
 * to delete. The markup is in git and in the prototype when they become real.
 */

export function Panes({
  onOpenFile,
}: {
  onOpenFile: (path: string) => void
}): React.JSX.Element {
  const { open, active, show, close, project } = useShell()

  return (
    <section className="mid">

        <div className="mid__body">
          <div className="panes" data-empty={String(open.length === 0)}>
            <div className="blank">
              <span className="blank__mark"><img className="mark" alt="" src={mark} /></span>
              <div className="blank__name">{project?.name ?? 'devpit'}</div>
              <div className="blank__sub">Nothing open. Pick something on the left, or start here.</div>
              <div className="blank__keys">
                <button className="blank__k" onClick={() => show('chat')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" /></svg>
                  <b>New chat</b><span>&#8984;N</span>
                </button>
                <button className="blank__k" onClick={() => show('term')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg>
                  <b>New terminal</b><span>&#8984;T</span>
                </button>
                <button className="blank__k" onClick={() => show('board')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg>
                  <b>Board</b><span>&#8984;B</span>
                </button>
              </div>
            </div>


            {/* Board */}
            <div className="pane" data-pane="board" data-show={String(active?.kind === 'board')}>
              <div className="pane__bar">
                <span className="pane__ico"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg></span>
                <span className="pane__t"><b>Board</b></span>
                <span className="drag"></span>
                <button className="sq26" onClick={() => close('board')} aria-label="Close board"><svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg></button>
              </div>
              <BoardPane />
            </div>

            {/* Skills */}
            <div className="pane" data-pane="skills" data-show={String(active?.kind === 'skills')}>
              <SkillsPane />
            </div>


            <div className="pane" data-pane="mcps" data-show={String(active?.kind === 'mcps')}>
              <McpPane />
            </div>

            <div className="pane" data-pane="files" data-show={String(active?.kind === 'files')}>
              <FilesPane onOpenFile={onOpenFile} />
            </div>

            <div className="pane" data-pane="workspace" data-show={String(active?.kind === 'workspace')}>
              <WorkspacePane />
            </div>



            {/* Chat — one pane per conversation, like the terminal */}
            {open
              .filter((tab) => tab.kind === 'chat')
              .map((tab) => (
                <div
                  key={tab.id}
                  className="pane"
                  data-pane="chat"
                  data-show={String(active?.id === tab.id)}
                >
                  <ChatPane tab={tab} />
                </div>
              ))}

            {/* A diff, one pane per file being read */}
            {open
              .filter((tab) => tab.kind === 'diff')
              .map((tab) => (
                <div
                  key={tab.id}
                  className="pane pane--file"
                  data-pane="diff"
                  data-show={String(active?.id === tab.id)}
                >
                  <DiffPane tab={tab} />
                </div>
              ))}

            {/* A file, one pane per tab */}
            {open
              .filter((tab) => tab.kind === 'file')
              .map((tab) => (
                <div
                  key={tab.id}
                  className="pane pane--file"
                  data-pane="file"
                  data-show={String(active?.id === tab.id)}
                >
                  <FilePane tab={tab} />
                </div>
              ))}

            {/* Terminal */}
            {open
              .filter((tab) => tab.kind === 'term')
              .map((tab) => (
                <div
                  key={tab.id}
                  className="pane pane--term"
                  data-pane="term"
                  data-show={String(active?.id === tab.id)}
                >
                  <TerminalPane tab={tab} />
                </div>
              ))}

          </div>
        </div>
      </section>
  )
}
