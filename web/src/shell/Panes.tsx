import { useEffect, useState } from 'react'

import mark from '../assets/brand/mark.png'
import { keptAfter, type KeptTab } from './kept'
import { PANE_MOUNTS } from './paneMounts'
import type { Tab } from './strip'
import { useShell } from './useShell'
import { SHORTCUTS } from './shortcuts'

export function Panes(): React.JSX.Element {
  const { open, active, show, close, project } = useShell()
  const here = project?.id ?? ''

  /* Tabs of the projects left behind, still mounted. Worked out after the
     render, and read without this project's own: those come from `open`,
     under the same keys, so a tab that moves from one list to the other is
     the same element and never mounts again. */
  const [kept, setKept] = useState<readonly KeptTab[]>([])
  useEffect(() => setKept((was) => keptAfter(was, here || null, open, keeps)), [here, open])
  const elsewhere = kept.filter((one) => one.projectId !== here)

  /* A pane drawn once — the files, the capabilities — mounts the first time
     it is opened, not at start: each reads and subscribes on mount, for a
     screen nobody may open. Once opened it stays. The board is the exception
     (`eager`): what it shows arrives only as events. */
  const [opened, setOpened] = useState<ReadonlySet<string>>(() => new Set())
  const activeKind = active?.kind
  useEffect(() => {
    if (activeKind && !opened.has(activeKind)) setOpened((was) => new Set(was).add(activeKind))
  }, [activeKind, opened])

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
                  <b>New chat</b><span>{SHORTCUTS.chat}</span>
                </button>
                <button className="blank__k" onClick={() => show('term')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="m4 17 6-6-6-6M12 19h8" /></svg>
                  <b>New terminal</b><span>{SHORTCUTS.terminal}</span>
                </button>
                <button className="blank__k" onClick={() => show('board')}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M9 3v18M15 3v18" /></svg>
                  <b>Board</b>
                </button>
              </div>
            </div>

            {PANE_MOUNTS.flatMap((mount) =>
              mount.many
                ? [
                    ...open.filter((tab) => tab.kind === mount.name).map((tab) => ({ projectId: here, tab })),
                    ...(mount.keep ? elsewhere.filter((one) => one.tab.kind === mount.name) : []),
                  ].map(({ projectId, tab }) => {
                    const shown = projectId === here && active?.id === tab.id
                    return (
                      <div key={tab.id} className={mount.className} data-pane={mount.name} data-show={String(shown)}>
                        {mount.render(tab, projectId)}
                      </div>
                    )
                  })
                : !mount.eager && !opened.has(mount.name) && active?.kind !== mount.name
                  ? []
                  : [
                      <div key={mount.name} className={mount.className} data-pane={mount.name} data-show={String(active?.kind === mount.name)}>
                        {mount.render(close)}
                      </div>,
                    ],
            )}

          </div>
        </div>
      </section>
  )
}

const keeps = (tab: Tab): boolean =>
  PANE_MOUNTS.some((mount) => mount.many && mount.keep === true && mount.name === tab.kind)
