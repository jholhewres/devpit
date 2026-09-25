import { useCallback, useEffect, useState } from 'react'

import type { Artifacts } from '../gen/bindings'
import { bytes } from './disk'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * The project's artifacts: files kept for it outside its repository, in its
 * own folder in the devpit workspace. Sessions keep and take them back with
 * devpit's tools; here they are read, revealed and removed.
 */
export function ArtifactsView({ shown }: { shown: boolean }): React.JSX.Element {
  const { project, show } = useShell()
  const [found, setFound] = useState<Artifacts | null>(null)
  const [error, setError] = useState<string | null>(null)
  /* Removing asks once, in place: the window has no dialog for one file. */
  const [removing, setRemoving] = useState<string | null>(null)

  const load = useCallback(() => {
    if (!project) return
    void ask(() => commands.artifactsList(project.id)).then((answer) => {
      setFound(answer.data ?? null)
      setError(answer.error)
    })
  }, [project])

  /* Sessions add to it while the panel is away; read again on the way in. */
  useEffect(() => {
    if (shown) load()
  }, [shown, load])

  const remove = (name: string): void => {
    if (!project) return
    void ask(() => commands.artifactRemove(project.id, name)).then((answer) => {
      setError(answer.error)
      setRemoving(null)
      load()
    })
  }

  return (
    <div className="arts">
      <div className="arts__top">
        <span className="arts__t">Artifacts</span>
        {found && (
          <button className="arts__act" onClick={() => void ask(() => commands.pathReveal(found.folder))} title={found.folder}>
            Reveal folder
          </button>
        )}
      </div>
      {error && <p className="arts__note">{error}</p>}
      {found?.items.length === 0 && (
        <p className="arts__note">
          Files kept for this project outside its repository. Ask a session to keep one here — it has devpit&rsquo;s artifact tools.
        </p>
      )}
      <ul className="arts__list">
        {found?.items.map((one) => {
          const path = `${found.folder}/${one.name}`
          return (
            <li className="arts__i" key={one.name}>
              <button className="arts__name" title={path} onClick={() => show('file', { id: `file:${path}`, path, title: one.name.split('/').pop() })}>
                {one.name}
              </button>
              <span className="arts__size">{bytes(one.bytes)}</span>
              {removing === one.name ? (
                <button className="arts__act arts__act--bad" onClick={() => remove(one.name)} onBlur={() => setRemoving(null)} autoFocus>
                  Remove?
                </button>
              ) : (
                <button className="arts__x" aria-label={`Remove ${one.name}`} title="Remove" onClick={() => setRemoving(one.name)}>
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
                </button>
              )}
            </li>
          )
        })}
      </ul>
    </div>
  )
}
