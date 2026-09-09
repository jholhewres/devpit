import { useState } from 'react'

import { useShell } from './useShell'

/*
 * "Remove" and "delete" are the same word to most people, and only one of them
 * is true here — so the body says which, and deleting the workspace is a
 * separate box you have to tick.
 */
export function RemoveProject({
  project,
  onClose,
  onConfirm,
}: {
  project: string
  onClose: () => void
  onConfirm: () => void
}): React.JSX.Element {
  const [wipe, setWipe] = useState(false)
  const { projects } = useShell()
  const row = projects.find((other) => other.id === project)
  const name = row?.name ?? project
  const live = row?.worktrees.length ?? 0

  return (
    <div
      className="ask"
      data-open="true"
      onClick={(event) => event.target === event.currentTarget && onClose()}
    >
      <div className="ask__box" role="dialog" aria-modal="true" aria-labelledby="askT">
        <h2 className="ask__t" id="askT">Remove &ldquo;{name}&rdquo;?</h2>
        <p className="ask__d">
          devpit stops listing this project. <b>The folder stays on disk</b> &mdash; your code,
          your git history and your worktrees are not touched.
        </p>

        <div className="ask__warn" hidden={live === 0}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 9v4M12 17v.01" />
            <path d="M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
          </svg>
          <span>
            {live} worktree{live === 1 ? '' : 's'} in it. Removing leaves{' '}
            {live === 1 ? 'it' : 'them'} on disk.
          </span>
        </div>

        <button
          className="ask__opt"
          role="checkbox"
          aria-checked={wipe}
          onClick={() => setWipe((was) => !was)}
        >
          <span className="box">
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
              <path d="M20 6 9 17l-5-5" />
            </svg>
          </span>
          <span>
            <span className="ask__ot">Also delete this project&rsquo;s devpit workspace</span>
            <span className="ask__od">
              The board, its cards and the per-project settings. This cannot be undone.
            </span>
          </span>
        </button>

        <div className="ask__row">
          <button className="btn" onClick={onClose}>Cancel</button>
          <button className="btn btn--danger" onClick={onConfirm}>Remove</button>
        </div>
      </div>
    </div>
  )
}
