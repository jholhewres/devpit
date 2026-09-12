import { useCallback, useEffect, useState } from 'react'

import type { Notices } from '../gen/bindings'
import { ask, commands } from './live'
import { onEvent } from './window'

/*
 * What the bell knows.
 *
 * Pushed, not polled: the backend emits when it writes one, because a run
 * ending and an agent stopping to wait are the two things people leave the
 * window for, and a timer finds out late and finds out repeatedly.
 *
 * The count comes back with the list from every command, so the badge and the
 * panel can never disagree — two separate reads a second apart is exactly how
 * a badge ends up saying 3 over an empty panel.
 */

export interface Bell {
  readonly notices: Notices['notices']
  readonly unread: number
  markRead: (id: string) => void
  markAllRead: () => void
  reload: () => void
}

export function useNotices(): Bell {
  const [seen, setSeen] = useState<Notices>({ notices: [], unread: 0 })

  const take = useCallback((answer: { data: Notices | null }) => {
    if (answer.data) setSeen(answer.data)
  }, [])

  const reload = useCallback(() => {
    void ask(() => commands.noticesRead()).then(take)
  }, [take])

  useEffect(() => {
    /* The deadline sweep runs once on open rather than on a timer: a date
       passing is not an event, so something has to look — but looking once a
       launch is enough for a thing that changes at midnight. */
    void ask(() => commands.noticesSweepDue()).then(take)
  }, [take])

  useEffect(() => onEvent('notice:rang', reload), [reload])

  return {
    notices: seen.notices,
    unread: seen.unread,
    markRead: (id) => void ask(() => commands.noticesMark(id)).then(take),
    markAllRead: () => void ask(() => commands.noticesMarkAll()).then(take),
    reload,
  }
}
