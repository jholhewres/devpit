import { useEffect, useState } from 'react'

import { ask, commands } from './live'
import { Markdown } from './MarkdownView'
import { ASIDE } from './slash'

/*
 * `/btw`: a side question, answered above the composer from what the
 * conversation already holds. The turn running goes on, and the
 * conversation's history never holds the question or its answer.
 */
export function SideAnswer({ question, projectId, conversationId, onClose }: { question: string; projectId: string | null; conversationId: string; onClose: () => void }): React.JSX.Element {
  const [said, setSaid] = useState<{ text: string | null; error: string | null } | null>(null)
  useEffect(() => {
    if (!question || !projectId) return
    let live = true
    void ask(() => commands.chatAside(projectId, conversationId, question)).then((answer) => live && setSaid({ text: answer.data ?? null, error: answer.error }))
    return () => {
      live = false
    }
  }, [question, projectId, conversationId])

  return (
    <section className="slash oans" aria-label="Side question">
      <div className="oans__top">
        <p className="oans__t">/{[ASIDE, question].filter(Boolean).join(' ')}</p>
        <button className="sq26" onClick={onClose} aria-label="Close" title="Close">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>
      <div aria-live="polite">
        {!question ? (
          <p className="sess__note">Ask something after /{ASIDE}: it is answered here, and the turn goes on.</p>
        ) : said === null ? (
          <p className="sess__note">Asking on the side…</p>
        ) : said.error ? (
          <p className="sess__note">{said.error}</p>
        ) : (
          <div className="oans__a">
            <Markdown source={said.text ?? ''} />
          </div>
        )}
      </div>
    </section>
  )
}
