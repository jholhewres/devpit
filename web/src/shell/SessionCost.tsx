import { useEffect, useState } from 'react'

import type { SessionCost as Cost } from '../gen/bindings'
import { ask, commands } from './live'
import { costDetail, costWords, keepUnit, unitNow, type CostUnit } from './spentWords'

const EVERY_MS = 3000

/*
 * What sessions have spent, read by one timer for every widget on screen — a
 * terminal's corner, a row, a chat tab — and each session once a tick however
 * many widgets show it. Every terminal pane stays mounted, and a timer each
 * was a read per pane every three seconds.
 */

interface Watch {
  cost: Cost | null
  /* What was last told, as text: a read that found the same is told to nobody. */
  said: string
  heard: Set<(cost: Cost | null) => void>
  /* The transcript is read on from where the last ask stopped; one at a time. */
  asking: boolean
}

const watches = new Map<string, Watch>()
let timer: number | null = null

function read(sessionId: string, watch: Watch): void {
  /* Nobody is looking at a hidden window; it is read again when it shows. */
  if (watch.asking || document.hidden) return
  watch.asking = true
  void ask(() => commands.sessionCost(sessionId)).then((answer) => {
    watch.asking = false
    const said = JSON.stringify(answer.data)
    if (said === watch.said) return
    watch.said = said
    watch.cost = answer.data
    for (const tell of watch.heard) tell(watch.cost)
  })
}

const readAll = (): void => watches.forEach((watch, sessionId) => read(sessionId, watch))

/* Back on screen, read at once rather than a tick late. */
const back = (): void => {
  if (!document.hidden) readAll()
}

/** Tells `tell` what a session has spent, now and as it changes, until the answer is called. */
function watchCost(sessionId: string, tell: (cost: Cost | null) => void): () => void {
  let found = watches.get(sessionId)
  if (!found) {
    found = { cost: null, said: '', heard: new Set(), asking: false }
    watches.set(sessionId, found)
  }
  const mine = found
  mine.heard.add(tell)
  if (mine.cost) tell(mine.cost)
  else read(sessionId, mine)
  if (timer === null) {
    timer = window.setInterval(readAll, EVERY_MS)
    document.addEventListener('visibilitychange', back)
  }
  return () => {
    mine.heard.delete(tell)
    if (mine.heard.size === 0 && watches.get(sessionId) === mine) watches.delete(sessionId)
    if (watches.size === 0 && timer !== null) {
      window.clearInterval(timer)
      timer = null
      document.removeEventListener('visibilitychange', back)
    }
  }
}

export function useSessionCost(sessionId: string | null | undefined): Cost | null {
  const [cost, setCost] = useState<Cost | null>(null)
  useEffect(() => {
    setCost(null)
    if (!sessionId) return
    return watchCost(sessionId, setCost)
  }, [sessionId])
  return cost
}

export function SessionCost({ sessionId, className = 'scost' }: { sessionId: string | null | undefined; className?: string }): React.JSX.Element | null {
  const cost = useSessionCost(sessionId)
  const [unit, setUnit] = useState<CostUnit>(unitNow)
  if (!cost) return null
  const flip = (): void => {
    const next = unit === 'usd' ? 'tokens' : 'usd'
    keepUnit(next)
    setUnit(next)
  }
  return (
    <button className={className} title={costDetail(cost)} onClick={flip}>
      {costWords(cost, unit)}
    </button>
  )
}
