import { useCallback } from 'react'

import { sweeping, type Strip, type Sweep } from './strip'

/*
 * Closing several tabs at once, as the tab's menu names it.
 *
 * Its own file because `useShell` is at its ceiling, and because the rule this
 * holds is worth stating once: it closes through whatever `close` it is given,
 * one tab at a time. In the shell that is the guarded close — the one that
 * asks before taking a terminal with work in it — and "Close all" is exactly
 * where the raw one would have been a mistake nobody noticed until it ate
 * something.
 */
export function useSweep(
  strip: Strip,
  close: (id: string) => void,
): (id: string, what: Sweep) => void {
  return useCallback(
    (id, what) => {
      for (const going of sweeping(strip, id, what)) close(going)
    },
    [strip, close],
  )
}
