import { useShell } from './useShell'

const Folder = (): React.JSX.Element => (
  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
  </svg>
)

/* The list, not a fixed set of names: a project added while the window is
   open has to appear here without a rebuild. */
export function ProjectRows({ onRemove }: { onRemove: (id: string) => void }): React.JSX.Element {
  const { projects, project, setProject, projectsError } = useShell()

  if (projectsError) return <p className="acc__note">{projectsError}</p>
  if (projects.length === 0) return <p className="acc__note">No projects yet.</p>

  return (
    <>
      {projects.map((row) => {
        const here = row.id === project?.id
        return (
          <div className="pj" key={row.id} data-live={row.worktrees.length}>
            <Folder />
            <span className="pj__b">
              <span className="pj__n">
                {row.name}
                {here && <span className="pj__here">Open</span>}
              </span>
              <span className="pj__p">{row.unreadable ?? row.rootPath}</span>
            </span>
            <span className="pj__live" data-live={row.worktrees.length}>
              <span className="prow__dot" />
              {row.worktrees.length} worktrees
            </span>
            <span className="pj__when">{row.unreadable ? 'unreadable' : ''}</span>
            <span className="pj__acts">
              {!here && (
                <button className="btn" onClick={() => setProject(row.id)}>
                  Open
                </button>
              )}
              <button className="btn" onClick={() => onRemove(row.id)}>
                Remove
              </button>
            </span>
          </div>
        )
      })}
    </>
  )
}
