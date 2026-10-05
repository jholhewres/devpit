import { useEffect, useState } from 'react'

import type { SessionCost as Cost } from '../gen/bindings'
import { ask, commands } from './live'
import { costDetail, costWords, keepUnit, unitNow, type CostUnit } from './sessionCost'

const EVERY_MS = 3000

/* What a session has spent, read again every few seconds while the window is
   seen: the transcript is read on from where the last ask stopped. */
export function useSessionCost(sessionId: string | null | undefined): Cost | null {
  const [cost, setCost] = useState<Cost | null>(null)
  useEffect(() => {
    setCost(null)
    if (!sessionId) return
    let gone = false
    const read = (): void => {
      if (document.hidden) return
      void ask(() => commands.sessionCost(sessionId)).then((answer) => !gone && setCost(answer.data))
    }
    read()
    const timer = window.setInterval(read, EVERY_MS)
    return () => {
      gone = true
      window.clearInterval(timer)
    }
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
