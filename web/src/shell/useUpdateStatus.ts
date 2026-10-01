import { useEffect, useState } from 'react'

import type { UpdateStatus } from '../gen/bindings'
import { commands } from './live'
import { onCarried } from './window'

/**
 * Where the update stands, as the app holds it, and every move after.
 *
 * Asked once because the check at startup can finish before anything listens,
 * and heard after that; an event that arrives before the first answer is
 * newer, and wins. The setter is for a caller that got a state back from a
 * command of its own.
 */
export function useUpdateStatus(): [UpdateStatus | null, (status: UpdateStatus) => void] {
  const [status, setStatus] = useState<UpdateStatus | null>(null)
  useEffect(() => {
    let heard = false
    const stop = onCarried<UpdateStatus>('update:status', (now) => {
      heard = true
      setStatus(now)
    })
    void commands
      .updateStatus()
      .then((now) => !heard && now.type !== 'idle' && setStatus(now))
      .catch(() => undefined)
    return stop
  }, [])
  return [status, setStatus]
}

/** Whether there is a newer devpit somewhere between found and installed. An
 *  offer from a test feed is not one: nothing it names can be installed. */
export function aNewerOne(status: UpdateStatus | null): boolean {
  switch (status?.type) {
    case 'available':
      return !status.testFeed
    case 'downloading':
    case 'ready':
    case 'waiting':
    case 'manualInstall':
      return true
    default:
      return false
  }
}
