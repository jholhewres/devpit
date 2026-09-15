import { useEffect, useState } from 'react'

import type { Installation, Profile, SessionHit } from '../gen/bindings'
import { ask, commands } from './live'
import { profileFor } from './outside'
import { pieces } from './snippet'
import { useShell } from './useShell'

/*
 * Finding a conversation of this project by something said in it.
 *
 * Every installation's transcripts, devpit's own and the ones started in a
 * terminal. A hit on a conversation devpit already holds opens that tab; any
 * other is taken in the same way a terminal session is.
 */

/* Typing is not a query per keystroke: the first look at a project reads its
   whole history. */
const WAIT_MS = 250

export function SessionSearch(): React.JSX.Element | null {
  const { project, show } = useShell()
  const [query, setQuery] = useState('')
  const [hits, setHits] = useState<readonly SessionHit[]>([])
  const [error, setError] = useState<string | null>(null)
  const [looking, setLooking] = useState(false)

  useEffect(() => {
    if (!project || !query.trim()) return setHits([])
    let current = true
    const timer = window.setTimeout(() => {
      setLooking(true)
      void ask(() => commands.sessionsSearch(project.id, query)).then((answer) => {
        if (!current) return
        setHits(answer.data ?? [])
        setError(answer.error)
        setLooking(false)
      })
    }, WAIT_MS)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [project, query])

  if (!project) return null

  const open = async (hit: SessionHit): Promise<void> => {
    if (hit.conversationId) return show('chat', { id: hit.conversationId })
    const [installed, known] = await Promise.all([
      ask(() => commands.cliInstallations()),
      ask(() => commands.agentProfiles()),
    ])
    const profile = profileFor(
      { sessionId: hit.sessionId, title: null, installation: hit.installation, lastAt: null },
      (installed.data ?? []) as readonly Installation[],
      (known.data ?? []) as readonly Profile[],
    )
    if (!profile) return setError('No profile runs against the installation that holds this session.')
    const adopted = await ask(() => commands.chatAdopt(project.id, hit.sessionId, profile.id, null, null))
    if (adopted.data) show('chat', { id: adopted.data })
    else setError(adopted.error)
  }

  return (
    <div className="ssearch">
      <input
        className="fb__find"
        placeholder="Search what was said…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        aria-label="Search conversations"
      />
      {looking && <div className="sessions__none">Reading transcripts…</div>}
      {error && <div className="sessions__none">{error}</div>}
      {!looking && query.trim() && hits.length === 0 && !error && <div className="sessions__none">Nothing said matches.</div>}
      {hits.map((hit, at) => (
        <button className="card" key={`${hit.sessionId}-${at}`} onClick={() => void open(hit)}>
          <span className="card__l2">
            <span className="card__loose">
              {pieces(hit.snippet).map((piece, index) =>
                piece.hit ? <mark key={index}>{piece.text}</mark> : piece.text,
              )}
            </span>
          </span>
        </button>
      ))}
    </div>
  )
}
