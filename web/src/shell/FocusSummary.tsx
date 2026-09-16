import { useEffect, useState } from 'react'

import type { HeadsDown, Notice } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * What the focus held, shown on the way out.
 *
 * This is the other half of the promise: nothing that arrived while the door
 * was shut is dropped, and it is not shown as a wall of rings the moment the
 * focus ends either — it is one list, grouped by the project it came from,
 * because three things from one subsystem read as one thing and three things
 * from three read as three.
 *
 * Read through `focus.waiting` rather than from the bell's own list: the bell
 * holds a page of the newest, and a focus is exactly the case where what
 * matters fell off the end of it.
 *
 * Nothing here is marked read by being shown. Acting on one is what reads it.
 */

/** The order somebody coming back wants: what is waiting on them, then what
 *  broke, then what is ready, then what is only telling them something. */
const RANK: Readonly<Record<string, number>> = { agent: 0, irreversible: 1, run: 2 }
const rankOf = (kind: string): number => RANK[kind] ?? 9

export function FocusSummary({
  ended,
  onOpenCard,
  onClose,
}: {
  ended: HeadsDown
  onOpenCard: (cardId: string) => void
  onClose: () => void
}): React.JSX.Element | null {
  const [waiting, setWaiting] = useState<Notice[]>([])
  const [more, setMore] = useState(false)
  const [read, setRead] = useState(false)

  useEffect(() => {
    if (ended.since === null) return
    let dropped = false
    void (async () => {
      /* Walked forward one page at a time, so a focus that lasted a day still
         shows every one exactly once. Bounded, because a summary nobody can
         scroll is a summary nobody reads. */
      const all: Notice[] = []
      let after: string | undefined
      let over = false
      for (let page = 0; page < 5; page += 1) {
        const answer = await ask(() =>
          commands.focusWaiting(ended.projectId, ended.since as number, after ?? null),
        )
        const heard = answer.data
        if (!heard || heard.notices.length === 0) break
        all.push(...heard.notices)
        after = heard.notices[heard.notices.length - 1]!.id
        over = heard.more
        if (!heard.more) break
      }
      if (dropped) return
      setWaiting(all)
      setMore(over)
      setRead(true)
    })()
    return () => {
      dropped = true
    }
  }, [ended])

  if (!read) return null

  const byProject = new Map<string, Notice[]>()
  for (const one of [...waiting].sort(
    (a, b) => rankOf(a.kind) - rankOf(b.kind) || (a.createdAt ?? 0) - (b.createdAt ?? 0),
  )) {
    const group = byProject.get(one.projectId ?? '') ?? []
    group.push(one)
    byProject.set(one.projectId ?? '', group)
  }

  return (
    <div className="fsum" role="dialog" aria-label="Focus ended">
      <div className="fsum__box">
        <div className="fsum__top">
          <span className="fsum__t">
            {waiting.length === 0
              ? 'Nothing arrived while you were in it'
              : `${waiting.length} arrived while you were in it${more ? ', and more behind them' : ''}`}
          </span>
          <button className="conv__act" onClick={onClose}>
            Done
          </button>
        </div>

        {[...byProject.entries()].map(([projectId, group]) => (
          <div className="fsum__grp" key={projectId || 'none'}>
            <span className="fsum__p">{projectId || 'No project'}</span>
            {group.map((one) => (
              <button
                className="fsum__one"
                key={one.id}
                data-kind={one.kind}
                onClick={() => {
                  if (one.cardId) onOpenCard(one.cardId)
                }}
              >
                <span className="fsum__line">{one.title}</span>
                {one.detail && <span className="fsum__d">{one.detail}</span>}
              </button>
            ))}
          </div>
        ))}
      </div>
    </div>
  )
}
