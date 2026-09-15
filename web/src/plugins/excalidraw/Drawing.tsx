import { lazy, Suspense, useEffect, useState } from 'react'

import type { Card } from '../../gen/bindings'
import { ControlMenu } from '../../shell/ControlMenu'
import { ask, commands } from '../../shell/live'
import { Rename } from '../../shell/Rename'
import type { Tab } from '../../shell/strip'
import { useShell } from '../../shell/useShell'
import { drawingTab, PLUGIN_ID, stemOf } from './drawings'
import { excalidrawTheme } from './theme'
import { useDrawing } from './useDrawing'

/* The editor is a megabyte with its own stylesheet; neither loads until a drawing opens. */
const Canvas = lazy(() => import('./Canvas').then((module) => ({ default: module.Canvas })))

const DARK = '(prefers-color-scheme: dark)'

function useSystemDark(): boolean {
  const [dark, setDark] = useState(() => window.matchMedia(DARK).matches)
  useEffect(() => {
    const query = window.matchMedia(DARK)
    const follow = (): void => setDark(query.matches)
    query.addEventListener('change', follow)
    return () => query.removeEventListener('change', follow)
  }, [])
  return dark
}

export function Drawing({ tab, name }: { tab: Tab; name: string }): React.JSX.Element {
  const { project, theme, show, close, markUnsaved } = useShell()
  const drawing = useDrawing(name)
  const dark = useSystemDark()
  const [renaming, setRenaming] = useState(false)
  const [cards, setCards] = useState<readonly Card[]>([])
  const [pinned, setPinned] = useState<string | null>(null)
  const stem = stemOf(name)

  useEffect(() => {
    if (!project) return
    void ask(() => commands.boardGet(project.id)).then((answer) => setCards(answer.data?.cards ?? []))
  }, [project])

  /* By name: the backend finds the file, so the page never hands it a path. */
  const pinTo = (cardId: string): void => {
    if (!project) return
    const title = cards.find((card) => card.id === cardId)?.title ?? 'the card'
    void ask(() => commands.pluginDataPin(project.id, PLUGIN_ID, name, cardId)).then((answer) =>
      setPinned(answer.data ? `Pinned to ${title}.` : answer.error),
    )
  }

  /* A refused save is the one state where closing the tab would lose the drawing. */
  useEffect(() => {
    markUnsaved(tab.id, drawing.conflict)
    return () => markUnsaved(tab.id, false)
  }, [tab.id, drawing.conflict, markUnsaved])

  const renamed = (typed: string | null): void => {
    setRenaming(false)
    if (typed === null) return
    void drawing.rename(typed).then((next) => {
      if (!next) return
      show('drawing', drawingTab(next))
      close(tab.id)
    })
  }

  return (
    <>
      <div className="pane__bar">
        <span className="pane__ico">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 19l7-7 3 3-7 7-3-3ZM18 13l-1.5-7.5L2 2l3.5 14.5L13 18ZM2 2l7.6 7.6" /></svg>
        </span>
        <span className="pane__t">
          <b><Rename value={stem} editing={renaming} onDone={renamed} /></b>
        </span>
        <span className="drag"></span>
        {/* No cards, no menu: an empty list reads as a broken one. */}
        {cards.length > 0 && (
          <ControlMenu
            label="Pin to card"
            title="Pin to card"
            choices={cards.map((card) => ({ id: card.id, label: card.title }))}
            onPick={pinTo}
            opens="down"
          />
        )}
        <button className="sq26" onClick={() => setRenaming(true)} disabled={drawing.conflict} aria-label="Rename drawing">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" /></svg>
        </button>
        <button className="sq26" onClick={() => close(tab.id)} aria-label="Close drawing">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      {drawing.conflict && (
        <div className="clash" role="alert">
          <span className="clash__t">{stem} changed on disk after it was opened, so this change was not saved.</span>
          <button className="btn" onClick={drawing.reload}>Reload</button>
          <button className="btn" onClick={drawing.keepMine}>Keep mine</button>
        </div>
      )}
      {drawing.problem && (
        <div className="clash">
          <span className="clash__t">{drawing.problem}</span>
        </div>
      )}
      {pinned && (
        <div className="clash" role="status">
          <span className="clash__t">{pinned}</span>
        </div>
      )}

      <div className="plg-excalidraw__stage">
        {drawing.text !== null && (
          <Suspense fallback={<div className="plg-excalidraw__loading" />}>
            <Canvas
              key={drawing.generation}
              initialText={drawing.text}
              theme={excalidrawTheme(theme, dark)}
              onChange={drawing.change}
            />
          </Suspense>
        )}
      </div>
    </>
  )
}
