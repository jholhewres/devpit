import { useState } from 'react'

import { mark } from './tree'
import { Tree } from './Tree'
import { useShell } from './useShell'
import { useTree } from './useTree'

const Icon = ({ d, size = 15 }: { d: string; size?: number }): React.JSX.Element => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
    <path d={d} />
  </svg>
)

const FOLDER = 'M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z'
const UPLOAD = 'M12 16V4M8 8l4-4 4 4M4 20h16'
const COLLAPSE = 'M4 7h16M4 12h16M4 17h16'
const REFRESH = 'M21 12a9 9 0 0 1-15.5 6.2M3 12a9 9 0 0 1 15.5-6.2M3 20v-5h5M21 4v5h-5'
const SEARCH = 'm21 21-4.3-4.3'

/* Explorer or Changes: two questions about the same tree, so one is answered
   at a time rather than both being half-visible. */
export function RightPanel({ onOpenFile }: { onOpenFile: (path: string) => void }): React.JSX.Element {
  const { project } = useShell()
  const tree = useTree(project?.id ?? null)
  const [view, setView] = useState<'tree' | 'changes'>('tree')
  const [mode, setMode] = useState<'names' | 'contents'>('names')
  const [query, setQuery] = useState('')
  const [flags, setFlags] = useState({ case: false, word: false, regex: false })
  const [collapsed, setCollapsed] = useState(0)
  const [current, setCurrent] = useState<string | null>(null)

  function open(path: string): void {
    setCurrent(path)
    onOpenFile(path)
  }

  return (
    <aside className="rp">
      <div className="rp__bar">
        <button className="rtab" aria-selected={view === 'tree'} onClick={() => setView('tree')}>
          <Icon d={FOLDER} size={14} />
          Explorer
        </button>
        <button className="rtab" aria-selected={view === 'changes'} onClick={() => setView('changes')}>
          <Icon d={UPLOAD} size={14} />
          Changes
          <span className="rtab__n">{tree.changes.length}</span>
        </button>
      </div>

      <div className="rview" data-rview="tree" data-open={String(view === 'tree')}>
        <div className="ex__head">
          <span className="ex__proj">{project?.name ?? 'No project'}</span>
          <button className="sq26 tip" data-tip="Collapse all" aria-label="Collapse all" onClick={() => setCollapsed((was) => was + 1)}>
            <Icon d={COLLAPSE} />
          </button>
          <button className="sq26 tip" data-tip="Refresh" aria-label="Refresh" onClick={tree.reload}>
            <Icon d={REFRESH} />
          </button>
        </div>

        <div className="ex__search">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="11" cy="11" r="8" />
            <path d={SEARCH} />
          </svg>
          <input
            className="ex__q"
            placeholder={mode === 'contents' ? 'Search in files' : 'Search'}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <span className="ex__flags">
            <button className="flag" aria-pressed={flags.case} title="Match case" onClick={() => setFlags((f) => ({ ...f, case: !f.case }))}>Aa</button>
            <button className="flag" aria-pressed={flags.word} title="Whole word" onClick={() => setFlags((f) => ({ ...f, word: !f.word }))}>ab</button>
            <button className="flag" aria-pressed={flags.regex} title="Regular expression" onClick={() => setFlags((f) => ({ ...f, regex: !f.regex }))}>.*</button>
          </span>
        </div>

        <div className="ex__modes">
          <button className="exmode__tab" data-ex="names" aria-selected={mode === 'names'} onClick={() => setMode('names')}>Names</button>
          <button className="exmode__tab" data-ex="contents" aria-selected={mode === 'contents'} onClick={() => setMode('contents')}>Contents</button>
        </div>

        {tree.error && <div className="exempty"><span className="exempty__t">{tree.error}</span></div>}
        {!tree.error && mode === 'names' && (
          <Tree projectId={project?.id ?? ''} nodes={tree.nodes} query={query} current={current} collapsed={collapsed} onOpen={open} />
        )}
        {!tree.error && mode === 'contents' && (
          <div className="exempty">
            <span className="exempty__t">Type to search in files</span>
            <span className="exempty__d">Searching file contents is not wired yet.</span>
          </div>
        )}
      </div>

      <div className="rview" data-rview="changes" data-open={String(view === 'changes')}>
        <div className="git__head">
          <span className="git__branch">Changes</span>
          <span className="git__up">
            <span className="add">+{tree.totals.added}</span>
            <span className="del">&minus;{tree.totals.removed}</span>
          </span>
          <button className="sq26 tip" data-tip="Refresh" aria-label="Refresh" onClick={tree.reload}>
            <Icon d={REFRESH} />
          </button>
        </div>
        <div className="git__body">
          {tree.changes.length === 0 && (
            <div className="exempty"><span className="exempty__t">Nothing changed.</span></div>
          )}
          {tree.changes.map((change) => (
            <button key={change.path} className="gitrow gitrow--file" data-ctx="file" onClick={() => open(change.path)}>
              <span className="gitrow__n">{change.path}</span>
              <span className="gitrow__end">
                {change.added > 0 && <span className="add">+{change.added}</span>}
                {change.removed > 0 && <span className="del">&minus;{change.removed}</span>}
                <span className={`row__g row__g--${change.status}`}>{mark(change.status)}</span>
              </span>
            </button>
          ))}
        </div>
      </div>
    </aside>
  )
}
