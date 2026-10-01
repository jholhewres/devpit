import { useCallback, useEffect, useState } from 'react'

import type { Reminder, Reminders } from '../gen/bindings'
import { ask, commands } from './live'
import { onCarried } from './window'

/**
 * The reminders that went off and nobody has dealt with yet.
 *
 * Asked once, and again whenever one goes off or is dealt with anywhere — on
 * the banner, from an agent, by a date moved on a card. The window coming
 * back to the front also rearms the app's clock: a machine that slept may
 * have passed a reminder while the thread waiting for it slept too.
 */
export function useReminders(): {
  reminders: readonly Reminder[]
  snooze: (cardId: string, until: number) => void
  done: (cardId: string) => void
} {
  const [reminders, setReminders] = useState<readonly Reminder[]>([])

  const told = useCallback((answer: { data: Reminders | null }): void => {
    if (answer.data) setReminders(answer.data.reminders)
  }, [])
  const reload = useCallback(() => void ask(() => commands.remindersPending()).then(told), [told])

  useEffect(() => {
    reload()
    const fired = onCarried<Reminders>('reminder:fired', reload)
    const changed = onCarried<null>('reminder:changed', reload)
    const back = (): void => {
      void commands.remindersRearm().catch(() => undefined)
    }
    window.addEventListener('focus', back)
    return () => {
      fired()
      changed()
      window.removeEventListener('focus', back)
    }
  }, [reload])

  return {
    reminders,
    snooze: (cardId, until) => void ask(() => commands.reminderSnooze(cardId, until)).then(told),
    done: (cardId) => void ask(() => commands.reminderDone(cardId)).then(told),
  }
}

/** The moments a reminder can be put off to, from `now`, in seconds. */
export function laterOptions(now: number): { label: string; until: number }[] {
  const tomorrow = new Date(now * 1000)
  tomorrow.setDate(tomorrow.getDate() + 1)
  tomorrow.setHours(9, 0, 0, 0)
  return [
    { label: '15 min', until: now + 15 * 60 },
    { label: '1 hour', until: now + 3600 },
    { label: 'Tomorrow', until: tomorrow.getTime() / 1000 },
  ]
}
