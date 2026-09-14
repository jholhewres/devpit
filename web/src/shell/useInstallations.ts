import { useCallback, useEffect, useState } from 'react'

import type { Installation } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * Which installation of the agent CLI a panel is showing.
 *
 * `null` until someone picks one, meaning "the default profile's" — the
 * backend answers that, so a panel never has to guess which directory the
 * default runs against.
 */

export interface Installations {
  readonly list: readonly Installation[]
  /** The directory picked, or null for the default profile's. */
  readonly chosen: string | null
  choose: (directory: string) => void
  reload: () => void
}

export function useInstallations(): Installations {
  const [list, setList] = useState<readonly Installation[]>([])
  const [chosen, setChosen] = useState<string | null>(null)

  const reload = useCallback(() => {
    void ask(() => commands.cliInstallations()).then((answer) => {
      const found = answer.data ?? []
      setList(found)
      /* A profile edited in Providers can take its directory away; a pick
         pointing at nothing would be refused on every read. */
      setChosen((was) => (was && found.some((one) => one.directory === was) ? was : null))
    })
  }, [])

  useEffect(reload, [reload])

  return { list, chosen, choose: setChosen, reload }
}

/** What to call an installation: its profiles, or its folder when none. */
export function named(installation: Installation): string {
  if (installation.profiles.length > 0) return installation.profiles.join(' · ')
  const folder = installation.directory.split('/').filter(Boolean).pop() ?? installation.directory
  return folder.replace(/^\.claude-?/, '') || 'claude'
}
