import { useEffect, useState } from 'react'

import type { Elsewhere } from '../gen/bindings'
import { ask, commands } from './live'
import { SHOW_PANEL } from './RightPanel'
import { useShell } from './useShell'

/*
 * What this card's sessions wrote outside its own checkout — another
 * repository its work reached into — which the diff above cannot show. Only
 * shown: that checkout is not the card's to manage.
 */

export function CardElsewhere({ cardId }: { cardId: string }): React.JSX.Element | null {
  const { setProject } = useShell()
  const [repos, setRepos] = useState<readonly Elsewhere[]>([])

  useEffect(() => {
    void ask(() => commands.cardElsewhere(cardId)).then((answer) => setRepos(answer.data ?? []))
  }, [cardId])

  if (repos.length === 0) return null
  return (
    <section className="cdiff cdiff--elsewhere" aria-label="Changed in other repositories">
      {repos.map((repo) => {
        const open = repo.files.filter((file) => file.uncommitted).length
        return (
          <details className="cdiff__f" key={repo.root}>
            <summary className="cdiff__s">
              <span className="cdiff__p" title={repo.root}>
                Also changed {repo.files.length} {repo.files.length === 1 ? 'file' : 'files'} in {repo.name}
              </span>
              <span className="cdiff__n">{open > 0 ? `${open} uncommitted` : 'all committed'}</span>
            </summary>
            <ul className="cdiff__else">
              {repo.files.map((file) => (
                <li key={file.path} data-uncommitted={file.uncommitted ? 'true' : undefined}>
                  {file.path}
                </li>
              ))}
            </ul>
            {repo.projectId && (
              <button className="btn" onClick={() => (setProject(repo.projectId!), window.dispatchEvent(new CustomEvent(SHOW_PANEL, { detail: 'changes' })))}>
                Open {repo.name}
              </button>
            )}
          </details>
        )
      })}
    </section>
  )
}
