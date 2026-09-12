import { useEffect, useState } from 'react'

import type { OpenApp } from '../gen/bindings'
import { ask, commands } from './live'

/* The apps this machine can hand a folder to.

   Fetched once per surface rather than once per row: the answer involves a
   shell, and one list of projects would otherwise ask it ten times. */
export function useOpeners(): readonly OpenApp[] {
  const [apps, setApps] = useState<readonly OpenApp[]>([])
  useEffect(() => {
    void ask(() => commands.appsList()).then((answer) => setApps(answer.data ?? []))
  }, [])
  return apps
}
