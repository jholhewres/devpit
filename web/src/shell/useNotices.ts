import { useCallback, useEffect, useState } from 'react'

import type { Notices } from '../gen/bindings'
import { held } from './headsDown'
import { ask, commands } from './live'
import { inOrder } from './inOrder'
import { useFocus } from './useHeadsDown'
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
  /** What a focus is holding back: from another project, since it began.
   *  Never dropped and never marked read — shown on the way out. */
  readonly waiting: Notices['notices']
  markRead: (id: string) => void
  markAllRead: () => void
  reload: () => void
}

export function useNotices(): Bell {
  const [seen, setSeen] = useState<Notices>({ notices: [], unread: 0 })
  /* Read here rather than passed in: the bell is the one place that has the
     whole list, so the split between what rings and what waits is made once,
     from one list, and the badge and the queue cannot disagree. */
  const focus = useFocus()

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

  const waiting = seen.notices.filter((one) => held(one, focus))
  const ringing = seen.notices.filter((one) => !held(one, focus))

  return {
    notices: ringing,
    /* The app's count, untouched, unless a focus is holding something back —
       then it is that count less what is waiting, because the badge and the
       panel must not disagree and the panel is no longer the whole list.
       Computing it here in every case would replace the app's answer with a
       guess about rows this page may not even hold. */
    unread: focus
      ? Math.max(0, seen.unread - waiting.filter((one) => one.readAt === null).length)
      : seen.unread,
    waiting,
    markRead: (id) => void inOrder('notices', () => ask(() => commands.noticesMark(id))).then(take),
    markAllRead: () => void ask(() => commands.noticesMarkAll()).then(take),
    reload,
  }
}
