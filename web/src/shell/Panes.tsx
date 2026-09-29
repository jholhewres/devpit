import { useEffect, useState } from 'react'

import { keptAfter, type KeptTab } from './kept'
import { PANE_MOUNTS } from './paneMounts'
import type { Tab } from './strip'
import { useShellPick } from './shellStore'
import { BlankPane } from './BlankPane'

export function Panes(): React.JSX.Element {
  // A slice: every pane is drawn from here, so this re-renders only for tabs.
  const { open, active, show, close, project } = useShellPick((all) => ({
    open: all.open,
    active: all.active,
    show: all.show,
    close: all.close,
    project: all.project,
  }))
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
            <BlankPane name={project?.name ?? 'devpit'} show={show} />

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
