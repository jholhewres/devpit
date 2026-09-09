import { useState } from 'react'

import { Tree } from './Tree'
import { useShell } from './useShell'
import { useTree } from './useTree'

/*
 * The project's files, full width.
 *
 * The same tree the right panel draws, from the same command — the rows here
 * used to be a fixed list of this repository's own files, which looked right
 * on this machine and was a fabrication on any other.
 */

export function FilesPane({
  onOpenFile,
}: {
  onOpenFile: (path: string) => void
}): React.JSX.Element {
  const { project, close } = useShell()
  const tree = useTree(project?.id ?? null)
  const [query, setQuery] = useState('')
  const [current, setCurrent] = useState<string | null>(null)

  const open = (path: string): void => {
    setCurrent(path)
    onOpenFile(path)
  }

  return (
    <>
      <div className="pane__bar">
        <span className="pane__t">
          <b>Files</b>
          {project ? ` · ${project.name}` : ''}
        </span>
        <span className="drag"></span>
        <button className="sq26" onClick={tree.reload} aria-label="Refresh">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 0 1-15.5 6.2M3 12a9 9 0 0 1 15.5-6.2M3 20v-5h5M21 4v5h-5" /></svg>
        </button>
        <button className="sq26" onClick={() => close('files')} aria-label="Close Files">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div className="scroll">
        {tree.error && <div className="exempty__t">{tree.error}</div>}
        <input
          className="sk__find"
          placeholder="Filter by name…"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          aria-label="Filter files"
        />
        {project && (
          <Tree
            projectId={project.id}
            nodes={tree.nodes}
            query={query}
            current={current}
            collapsed={0}
            onOpen={open}
          />
        )}
      </div>
    </>
  )
}
